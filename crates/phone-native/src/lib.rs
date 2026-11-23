//! Shared native host. Desktop, Android and iOS supply only their lifetime,
//! renderer, sandbox directory and replaceable OS integration provider.
mod asset_store;
use phone_host::native::background;
mod backup;
mod documents;
mod environment;
mod file_picker;
mod location;
mod map_cache;
mod media_files;
mod native_media;
mod network;
mod notifications;
mod operations;
mod picker_io;
mod pim;
mod platform_services;
mod services;
pub mod storage;

use ferroui_base::{
    interactivity::RoutedEventHandlerToken,
    reactive::IDisposable,
    threading::{DispatcherPriority, DispatcherTimer},
    Ref, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use phone_host::{platform::*, HostRequest};
use phone_runtime::PhoneRuntime;
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

pub use phone_ui::AppRegistration;
thread_local! { static MOUNTED: Cell<bool> = const { Cell::new(false) }; }
struct MountReservation {
    committed: bool,
}
impl MountReservation {
    fn acquire() -> Result<Self, String> {
        if MOUNTED.with(|mounted| mounted.replace(true)) {
            return Err("A phone host is already mounted on this UI thread.".into());
        }
        Ok(Self { committed: false })
    }
}
impl Drop for MountReservation {
    fn drop(&mut self) {
        if !self.committed {
            MOUNTED.with(|mounted| mounted.set(false));
        }
    }
}

pub struct NativeHost {
    runtime: Rc<PhoneRuntime>,
    view: Ref<Control>,
    media: Rc<native_media::NativeMedia>,
    environment: Option<environment::NativeEnvironment>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    layout: RefCell<Option<Rc<dyn IDisposable>>>,
    navigation: RefCell<Option<(WeakRef<TopLevel>, RoutedEventHandlerToken)>>,
    suspended: Cell<bool>,
    closed: Cell<bool>,
    closing: Cell<bool>,
    alarm_haptics: Cell<bool>,
    last_haptic: Cell<Option<Instant>>,
    activations: RefCell<Vec<String>>,
}
impl NativeHost {
    pub fn new(path: PathBuf, platform: Rc<dyn PlatformPlugin>) -> Result<Rc<Self>, String> {
        Self::with_services(path, platform, |_| Ok(()))
    }
    /// Add or explicitly replace service families before attaching the host.
    /// A replacement receives the same typed request contract as the defaults.
    pub fn with_services(
        path: PathBuf,
        platform: Rc<dyn PlatformPlugin>,
        configure: impl FnOnce(&mut ServiceRegistry) -> Result<(), String>,
    ) -> Result<Rc<Self>, String> {
        Self::compose(path, platform, Vec::new(), configure)
    }
    /// Add SDK applications and replace host service families at composition.
    /// The pinned shell supports one mounted phone per UI thread.
    pub fn compose(
        path: PathBuf,
        platform: Rc<dyn PlatformPlugin>,
        apps: Vec<AppRegistration>,
        configure: impl FnOnce(&mut ServiceRegistry) -> Result<(), String>,
    ) -> Result<Rc<Self>, String> {
        let mut mount = MountReservation::acquire()?;
        platform.state().validate()?;
        if !path.is_absolute() {
            return Err("Native phone data requires an absolute application-owned path.".into());
        }
        let (data, warning, save_path) = storage::load(&path);
        storage::set_active_data_path(save_path.clone());
        let vm = PhoneViewModel::new(
            data,
            Rc::new(move |data| {
                if let Err(error) = storage::save(&save_path, data) {
                    eprintln!("Could not save phone data: {error}");
                }
            }),
        );
        Self::clock(&vm);
        if let Some(warning) = warning {
            vm.set_status(warning);
        }
        let environment = if cfg!(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux"
        )) {
            Some(environment::NativeEnvironment::new(&vm))
        } else {
            None
        };
        phone_ui::set_preview_capture_enabled(true);
        media_files::install_image_loader();
        let view = phone_ui::create_view_with_apps(vm.clone(), apps)?;
        let media = native_media::NativeMedia::install(&vm, &view, platform.clone());
        let slot: Rc<RefCell<Weak<Self>>> = Rc::default();
        let receiver = slot.clone();
        let handler: Rc<dyn Fn(HostRequest) -> Result<(), String>> = Rc::new(move |request| {
            receiver
                .borrow()
                .upgrade()
                .ok_or("The native host is closed.")?
                .handle(request)
        });
        let mut registry = ServiceRegistry::default();
        for service in [
            HostService::Network,
            HostService::Files,
            HostService::Navigation,
            HostService::Clipboard,
            HostService::Media,
            HostService::Camera,
            HostService::Notifications,
            HostService::Location,
            HostService::Sharing,
            HostService::Environment,
            HostService::Updates,
        ] {
            registry.register("ferroui.native", service, handler.clone())?;
        }
        configure(&mut registry)?;
        let runtime = PhoneRuntime::attach(vm, platform, registry)?;
        operations::install(&runtime);
        let host = Rc::new(Self {
            runtime,
            view,
            media,
            environment,
            timer: RefCell::new(None),
            layout: RefCell::new(None),
            navigation: RefCell::new(None),
            suspended: Cell::new(false),
            closed: Cell::new(false),
            closing: Cell::new(false),
            alarm_haptics: Cell::new(false),
            last_haptic: Cell::new(None),
            activations: RefCell::default(),
        });
        *slot.borrow_mut() = Rc::downgrade(&host);
        mount.committed = true;
        let weak = Rc::downgrade(&host);
        *host.layout.borrow_mut() = Some(
            host.view
                .layout_updated(move || {
                    if let Some(host) = weak.upgrade() {
                        host.attach_navigation();
                        let bounds = host.view.bounds();
                        if bounds.width > 0.0 && bounds.height > 0.0 {
                            host.vm().set_viewport(bounds.width, bounds.height);
                            host.drain_activations();
                        }
                    }
                })
                .detach(),
        );
        let weak = Rc::downgrade(&host);
        *host.timer.borrow_mut() = Some(
            DispatcherTimer::run(
                move || {
                    let Some(host) = weak.upgrade().filter(|host| !host.closed.get()) else {
                        return false;
                    };
                    if !host.suspended.get() {
                        host.tick();
                    }
                    true
                },
                Duration::from_secs(1),
                DispatcherPriority::BACKGROUND,
            )
            .detach(),
        );
        Ok(host)
    }
    pub fn view(&self) -> Ref<Control> {
        self.view.clone()
    }
    pub fn vm(&self) -> Rc<PhoneViewModel> {
        self.runtime.vm()
    }
    pub fn runtime(&self) -> Rc<PhoneRuntime> {
        self.runtime.clone()
    }
    fn attach_navigation(self: &Rc<Self>) {
        if self.closed.get() {
            return;
        }
        let top = TopLevel::get_top_level(Some(&self.view));
        if self
            .navigation
            .borrow()
            .as_ref()
            .is_some_and(|(previous, _)| {
                top.as_ref().is_some_and(|top| {
                    previous
                        .upgrade()
                        .is_some_and(|previous| previous.ptr_eq(top))
                })
            })
        {
            return;
        }
        self.detach_navigation();
        let Some(top) = top else {
            return;
        };
        let weak = Rc::downgrade(self);
        let token = top.back_requested(move |_, event| {
            let Some(host) = weak
                .upgrade()
                .filter(|host| !host.closed.get() && !host.suspended.get())
            else {
                return;
            };
            if event.handled() {
                return;
            }
            let vm = host.vm();
            let role = host.runtime.platform().state();
            if phone_ui::has_overlay() || vm.route() != phone_domain::Route::Start {
                phone_ui::back(&vm);
                event.set_handled(true);
            } else if role.mode == HostMode::Launcher || role.launcher == HostRole::Active {
                // An active HOME must remain reachable when Back reaches Start.
                event.set_handled(true);
            }
        });
        *self.navigation.borrow_mut() = Some((top.downgrade(), token));
    }
    fn detach_navigation(&self) {
        if let Some((top, token)) = self.navigation.borrow_mut().take() {
            if let Some(top) = top.upgrade() {
                top.remove_handler(TopLevel::back_requested_event(), token);
            }
        }
    }
    pub fn validate_activation(uri: &str) -> Result<(), String> {
        phone_runtime::activation::parse(uri).map(|_| ())
    }
    pub fn activate_uri(&self, uri: &str) -> Result<(), String> {
        Self::validate_activation(uri)?;
        if self.closed.get() {
            return Err("The native host is closed.".into());
        }
        if phone_ui::host_ready(&self.vm()) {
            self.runtime.activate_uri(uri)
        } else {
            let mut pending = self.activations.borrow_mut();
            if pending.len() >= 16 {
                return Err("Too many pending app activations.".into());
            }
            pending.push(uri.to_owned());
            Ok(())
        }
    }
    fn drain_activations(&self) {
        if !phone_ui::host_ready(&self.vm()) || self.closed.get() {
            return;
        }
        let activations = std::mem::take(&mut *self.activations.borrow_mut());
        for uri in activations {
            if let Err(error) = self.runtime.activate_uri(&uri) {
                self.vm().set_status(error);
            }
        }
    }
    pub fn lifecycle(&self, event: HostLifecycle) {
        if self.closed.get() || self.closing.get() {
            return;
        }
        if event == HostLifecycle::Closing {
            self.closing.set(true);
        }
        self.suspended.set(event != HostLifecycle::Resumed);
        self.runtime.lifecycle(event);
        match event {
            HostLifecycle::Resumed => self.tick(),
            HostLifecycle::Suspended => {
                // This iOS bundle intentionally does not claim background audio.
                #[cfg(target_os = "ios")]
                if let Err(error) = self.media.pause_for_suspension() {
                    self.vm().set_status(error);
                }
            }
            HostLifecycle::Closing => {
                self.closed.set(true);
                self.detach_navigation();
                self.alarm_haptics.set(false);
                self.last_haptic.set(None);
                self.activations.borrow_mut().clear();
                self.media.close();
                if let Some(top) = TopLevel::get_top_level(Some(&self.view)) {
                    phone_host::native::notifications::close(notifications::window_handle(&top));
                }
                if let Some(timer) = self.timer.borrow_mut().take() {
                    timer.dispose();
                }
                MOUNTED.with(|mounted| mounted.set(false));
            }
        }
    }
    fn clock(vm: &PhoneViewModel) {
        let now = chrono::Local::now();
        vm.set_clock(now.timestamp());
        vm.set_utc_offset(now.offset().local_minus_utc());
    }
    fn tick(&self) {
        if self.closed.get() || self.suspended.get() {
            return;
        }
        self.drain_activations();
        let vm = self.vm();
        Self::clock(&vm);
        if self.closed.get() {
            return;
        }
        if let Some(environment) = &self.environment {
            environment.tick(&vm);
        }
        self.runtime.observe();
        if self.closed.get() {
            return;
        }
        phone_ui::host_tick(vm.now());
        if self.closed.get() {
            return;
        }
        phone_ui::alarm_tick(&vm);
        self.pulse_alarm();
    }
    fn pulse_alarm(&self) {
        if self.closed.get()
            || self.suspended.get()
            || !self.alarm_haptics.get()
            || self
                .last_haptic
                .get()
                .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
            || !self
                .runtime
                .platform()
                .state()
                .supports(HostFeature::Haptics)
        {
            return;
        }
        self.last_haptic.set(Some(Instant::now()));
        // Foreground, bounded tactile pulses. Unsupported or busy feedback must
        // not interfere with the separate audible alarm and its dismissal.
        let _ = self.runtime.dispatch_platform(
            PlatformRequest::Haptic {
                kind: HapticKind::Heavy,
            },
            Some(Box::new(|_, _| Ok(()))),
        );
    }
    fn handle(&self, request: HostRequest) -> Result<(), String> {
        if self.closed.get() {
            return Err("The native host is closed.".into());
        }
        if let HostRequest::AlarmAlert {
            active, vibration, ..
        } = &request
        {
            let enabled = *active && *vibration;
            if self.alarm_haptics.replace(enabled) != enabled {
                self.last_haptic.set(None);
            }
            self.pulse_alarm();
        }
        if platform_services::handle(&self.runtime, &request)? {
            return Ok(());
        }
        // Network/media cancellation does not require an attached visual root.
        match request {
            HostRequest::HttpGet {
                request_id,
                url,
                max_bytes,
            } => {
                network::start(request_id, url, max_bytes);
                Ok(())
            }
            HostRequest::MapTile {
                request_id,
                zoom,
                x,
                y,
            } => {
                network::map_tile(request_id, zoom, x, y);
                Ok(())
            }
            request @ (HostRequest::PlayMedia { .. }
            | HostRequest::MediaControl { .. }
            | HostRequest::StartCamera
            | HostRequest::StopCamera
            | HostRequest::CapturePhoto
            | HostRequest::AlarmAlert { .. }) => native_media::handle(request),
            request => {
                let top = TopLevel::get_top_level(Some(&self.view))
                    .ok_or("The native view is not attached to a platform surface.")?;
                services::handle(&self.vm(), &top, request)
            }
        }
    }
}
impl Drop for NativeHost {
    fn drop(&mut self) {
        self.lifecycle(HostLifecycle::Closing);
        if let Some(subscription) = self.layout.get_mut().take() {
            subscription.dispose();
        }
    }
}
