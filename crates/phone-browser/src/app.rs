use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use phone_domain::PhoneData;
use phone_host::platform::{HostService, PlatformPlugin, ServiceRegistry};
use phone_runtime::PhoneRuntime;
use phone_state::PhoneViewModel;
use std::{cell::RefCell, rc::Rc};
ferroui_base::ferro_thread_local! {
    static MODEL:RefCell<Option<Rc<PhoneViewModel>>> = const {RefCell::new(None)};
    static RUNTIME:RefCell<Option<Rc<PhoneRuntime>>> = const {RefCell::new(None)};
    static PLATFORM:RefCell<Option<Rc<dyn PlatformPlugin>>> = const {RefCell::new(None)};
    static STARTUP_ERROR:RefCell<Option<String>> = const {RefCell::new(None)};
    static ACTIVATIONS:RefCell<Vec<String>> = const {RefCell::new(Vec::new())};
}
pub fn prepare_platform(platform: Rc<dyn PlatformPlugin>) {
    PLATFORM.with(|slot| *slot.borrow_mut() = Some(platform));
}
pub fn startup_result() -> Result<(), String> {
    STARTUP_ERROR.with(|error| error.borrow_mut().take().map_or(Ok(()), Err))
}
pub fn runtime() -> Option<Rc<PhoneRuntime>> {
    RUNTIME.with(|runtime| runtime.borrow().clone())
}
pub fn activate(uri: &str) -> Result<(), String> {
    phone_runtime::activation::parse(uri)?;
    let runtime = runtime()
        .filter(|runtime| !runtime.is_closed())
        .ok_or("The browser host is not running.")?;
    if phone_ui::host_ready(&runtime.vm()) {
        return runtime.activate_uri(uri);
    }
    ACTIVATIONS.with(|pending| {
        let mut pending = pending.borrow_mut();
        if pending.len() >= 16 {
            return Err("Too many pending application activations.".into());
        }
        pending.push(uri.to_owned());
        Ok(())
    })
}
pub fn drain_activations() {
    let Some(runtime) =
        runtime().filter(|runtime| !runtime.is_closed() && phone_ui::host_ready(&runtime.vm()))
    else {
        return;
    };
    let pending = ACTIVATIONS.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
    for uri in pending {
        if let Err(error) = runtime.activate_uri(&uri) {
            runtime.vm().set_status(error);
        }
    }
}
pub fn close_runtime() {
    if let Some(runtime) = RUNTIME.with(|runtime| runtime.borrow_mut().take()) {
        runtime.lifecycle(phone_host::platform::HostLifecycle::Closing);
    }
    ACTIVATIONS.with(|pending| pending.borrow_mut().clear());
    if let Some(lifetime) =
        Application::current().and_then(|application| application.application_lifetime())
    {
        if let Some(lifetime) = lifetime.as_single_view_application_lifetime() {
            lifetime.set_main_view(None);
        }
    }
    MODEL.with(|model| model.borrow_mut().take());
}
pub fn model() -> Option<Rc<PhoneViewModel>> {
    MODEL.with(|m| m.borrow().clone())
}
#[repr(C)]
pub struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        phone_controls::install_theme(this);
    }
    fn on_framework_initialization_completed(this: &Self) {
        let json = crate::host::load();
        let mut warning = None;
        let data = match json {
            Ok(json) if json.trim().is_empty() => PhoneData::demo(),
            Ok(json) => PhoneData::from_json(&json).unwrap_or_else(|error| {
                crate::host::preserve_recovery(&json);
                warning = Some(format!(
                    "Saved data needs recovery. The original was preserved. {error}"
                ));
                PhoneData::demo()
            }),
            Err(_) => {
                warning =
                    Some("Storage is unavailable. Export a backup to keep your changes.".into());
                PhoneData::demo()
            }
        };
        let vm = PhoneViewModel::new(
            data,
            Rc::new(|data| {
                if let Ok(json) = data.to_json() {
                    if let Err(error) = crate::host::save(&json) {
                        crate::host::storage_error(&format!("{error:?}"));
                    }
                }
            }),
        );
        vm.set_clock(crate::host::now() as i64);
        vm.set_utc_offset(crate::host::utc_offset());
        vm.set_viewport(crate::host::width(), crate::host::height());
        let Some(platform) = PLATFORM.with(|slot| slot.borrow_mut().take()) else {
            STARTUP_ERROR.with(|error| {
                *error.borrow_mut() = Some("The browser platform was not prepared.".into())
            });
            return;
        };
        let mut services = ServiceRegistry::default();
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
            if let Err(error) = services.register(
                "ferroui.browser",
                service,
                Rc::new(|request| {
                    let json =
                        serde_json::to_string(&request).map_err(|error| error.to_string())?;
                    crate::host::request(&json);
                    Ok(())
                }),
            ) {
                STARTUP_ERROR.with(|slot| *slot.borrow_mut() = Some(error));
                return;
            }
        }
        let runtime = match PhoneRuntime::attach(vm.clone(), platform, services) {
            Ok(runtime) => runtime,
            Err(error) => {
                STARTUP_ERROR.with(|slot| *slot.borrow_mut() = Some(error));
                return;
            }
        };
        if let Some(warning) = warning {
            vm.set_status(warning);
        }
        MODEL.with(|slot| *slot.borrow_mut() = Some(vm.clone()));
        RUNTIME.with(|slot| *slot.borrow_mut() = Some(runtime));
        phone_ui::set_image_loader(Rc::new(|key| crate::host::load_image(&key)));
        phone_ui::set_preview_capture_enabled(true);
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            lifetime.set_main_view(Some(phone_ui::create_view(vm)));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
