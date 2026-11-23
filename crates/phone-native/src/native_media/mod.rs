//! Host-owned native media sessions; app crates only request shared services.
mod alarm;
mod camera;
mod handoff;
mod photo;
mod playback;
mod providers;
mod surfaces;
mod system_media;

use ferroui_base::{
    reactive::Subscription,
    rendering::composition::{frames::FrameBudget, ElementComposition, ICompositionGpuInterop},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    Ref, WeakRef,
};
use ferroui_controls::Control;
use ferroui_media_camera::{CameraCapture, CameraSession};
use ferroui_media_controls::TexturePresenter;
use ferroui_media_video::MediaPlayer;
use phone_state::{HostRequest, PhoneViewModel};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

thread_local! { static CURRENT: RefCell<Weak<NativeMedia>> = const { RefCell::new(Weak::new()) }; }

pub struct NativeMedia {
    vm: Weak<PhoneViewModel>,
    closed: Cell<bool>,
    platform: Rc<dyn phone_host::platform::PlatformPlugin>,
    window: WeakRef<Control>,
    budget: FrameBudget,
    player: RefCell<Option<MediaPlayer>>,
    alarm_player: RefCell<Option<MediaPlayer>>,
    camera_capture: RefCell<Option<CameraCapture>>,
    camera_session: RefCell<Option<CameraSession>>,
    video_views: RefCell<Vec<surfaces::Surface>>,
    camera_views: RefCell<Vec<surfaces::Surface>>,
    camera_generation: Cell<u64>,
    camera_pending: Cell<bool>,
    photo_pending: Cell<bool>,
    media_generation: Cell<u64>,
    alarm_generation: Cell<u64>,
    document_epoch: Cell<u64>,
    source: RefCell<String>,
    title: RefCell<String>,
    playing: Cell<bool>,
    desired_playing: Cell<bool>,
    media_stopped: Cell<bool>,
    pending_play_intent: Cell<Option<bool>>,
    media_ended: Cell<bool>,
    media_seek_generation: Cell<u64>,
    media_last_progress: Cell<Instant>,
    os_session: RefCell<Option<Box<dyn phone_host::native::media_backend::MediaBackend>>>,
    os_attempted: Cell<bool>,
    os_generation: Cell<u64>,
    os_seek_serial: Cell<u64>,
    os_last_sync: Cell<Instant>,
    audio_options: Cell<Option<(u8, bool)>>,
    timer: RefCell<Option<Subscription>>,
}

impl NativeMedia {
    pub fn install(
        vm: &Rc<PhoneViewModel>,
        window: &Ref<Control>,
        platform: Rc<dyn phone_host::platform::PlatformPlugin>,
    ) -> Rc<Self> {
        let state = Rc::new(Self {
            vm: Rc::downgrade(vm),
            closed: Cell::new(false),
            platform,
            window: window.downgrade(),
            budget: FrameBudget::new(Default::default()),
            player: RefCell::new(None),
            alarm_player: RefCell::new(None),
            camera_capture: RefCell::new(None),
            camera_session: RefCell::new(None),
            video_views: RefCell::new(Vec::new()),
            camera_views: RefCell::new(Vec::new()),
            camera_generation: Cell::new(0),
            camera_pending: Cell::new(false),
            photo_pending: Cell::new(false),
            media_generation: Cell::new(0),
            alarm_generation: Cell::new(0),
            document_epoch: Cell::new(vm.document_epoch()),
            source: RefCell::new(String::new()),
            title: RefCell::new(String::new()),
            playing: Cell::new(false),
            desired_playing: Cell::new(false),
            media_stopped: Cell::new(true),
            pending_play_intent: Cell::new(None),
            media_ended: Cell::new(false),
            media_seek_generation: Cell::new(0),
            media_last_progress: Cell::new(Instant::now()),
            os_session: RefCell::new(None),
            os_attempted: Cell::new(false),
            os_generation: Cell::new(1),
            os_seek_serial: Cell::new(0),
            os_last_sync: Cell::new(Instant::now()),
            audio_options: Cell::new(None),
            timer: RefCell::new(None),
        });
        CURRENT.with(|current| *current.borrow_mut() = Rc::downgrade(&state));
        let weak = Rc::downgrade(&state);
        phone_ui::set_host_surface_factory(Rc::new(move |kind| {
            weak.upgrade()
                .filter(|state| !state.closed.get())
                .map(|state| state.surface(kind))
        }));
        let weak = Rc::downgrade(&state);
        *state.timer.borrow_mut() = Some(DispatcherTimer::run(
            move || {
                let Some(state) = weak.upgrade() else {
                    return false;
                };
                state.tick();
                true
            },
            Duration::from_millis(33),
            DispatcherPriority::BACKGROUND,
        ));
        state
    }

    async fn interop(&self) -> Result<Rc<dyn ICompositionGpuInterop>, String> {
        let window = self.window.upgrade().ok_or("The phone window is closed.")?;
        let visual = ElementComposition::get_element_visual(&window)
            .ok_or("The native renderer is not ready.")?;
        visual
            .compositor()
            .try_get_composition_gpu_interop()
            .await
            .ok_or_else(|| "This renderer does not provide native media textures.".into())
    }

    fn status(&self, error: impl Into<String>) {
        if let Some(vm) = self.vm.upgrade() {
            vm.set_status(error.into());
        }
    }

    fn tick(self: &Rc<Self>) {
        if self.closed.get() {
            return;
        }
        let Some(vm) = self.vm.upgrade() else {
            return;
        };
        if self.document_epoch.replace(vm.document_epoch()) != vm.document_epoch() {
            self.stop_all();
        }
        if (self.camera_pending.get()
            || self.camera_capture.borrow().is_some()
            || self.camera_session.borrow().is_some())
            && !self.has_camera_surface()
        {
            self.stop_camera();
        }
        self.poll_playback(&vm);
        if self.closed.get() {
            return;
        }
        self.poll_system_media(&vm);
        if self.closed.get() {
            return;
        }
        self.poll_camera();
        if self.closed.get() {
            return;
        }
        self.poll_alarm();
    }

    #[cfg(target_os = "ios")]
    pub fn pause_for_suspension(self: &Rc<Self>) -> Result<(), String> {
        self.stop_camera();
        self.control("pause")
    }

    /// Stop polling and release only this owner's resources exactly once. A
    /// retained closed view must never publish into a replacement phone session.
    pub fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
        self.stop_all();
        self.os_session.borrow_mut().take();
    }

    pub fn stop_all(&self) {
        self.stop_camera();
        self.media_generation
            .set(self.media_generation.get().wrapping_add(1));
        self.alarm_generation
            .set(self.alarm_generation.get().wrapping_add(1));
        if let Some(player) = self.player.borrow_mut().take() {
            player.clear_source();
        }
        if let Some(player) = self.alarm_player.borrow_mut().take() {
            player.clear_source();
        }
        self.set_playing(false);
        self.desired_playing.set(false);
        self.media_stopped.set(true);
        self.source.borrow_mut().clear();
        self.title.borrow_mut().clear();
        phone_ui::show_media(String::new(), String::new(), false);
        self.clear_system_media();
        self.refresh_video_surfaces();
    }
}

impl Drop for NativeMedia {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn handle(request: HostRequest) -> Result<(), String> {
    let state = CURRENT
        .with(|current| current.borrow().upgrade())
        .filter(|state| !state.closed.get())
        .ok_or("Native media is not initialized or has closed.")?;
    match request {
        HostRequest::PlayMedia { source } => state.play(source),
        HostRequest::MediaControl { action } => state.control(&action),
        HostRequest::StartCamera => {
            state.start_camera();
            Ok(())
        }
        HostRequest::CapturePhoto => state.capture_photo(),
        HostRequest::StopCamera => {
            state.stop_camera();
            Ok(())
        }
        HostRequest::AlarmAlert {
            active,
            volume,
            vibration: _,
        } => {
            state.alarm(active, volume);
            Ok(())
        }
        _ => Err("This is not a native media request.".into()),
    }
}

/// Stop a removed library item before deleting its owned file, without stopping
/// an independent alarm or camera session. Remote sources own no local asset.
pub fn forget_source(source: &str) {
    let state = CURRENT.with(|current| current.borrow().upgrade());
    let Some(state) = state.filter(|state| state.source.borrow().as_str() == source) else {
        return;
    };
    state
        .media_generation
        .set(state.media_generation.get().wrapping_add(1));
    if let Some(player) = state.player.borrow_mut().take() {
        player.clear_source();
    }
    state.source.borrow_mut().clear();
    state.title.borrow_mut().clear();
    state.playing.set(false);
    state.desired_playing.set(false);
    state.media_stopped.set(true);
    phone_ui::show_media(String::new(), String::new(), false);
    phone_ui::media_progress(0.0, 0.0);
    state.clear_system_media();
    state.refresh_video_surfaces();
}
