use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_impl_classes, instantiate};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use phone_domain::PhoneData;
use phone_state::{HostServices, PhoneViewModel};
use std::{cell::RefCell, rc::Rc};
ferroui_base::ferro_thread_local! {
    static MODEL:RefCell<Option<Rc<PhoneViewModel>>> = const {RefCell::new(None)};
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
        vm.set_services(HostServices {
            request: Rc::new(|request| {
                if let Ok(json) = serde_json::to_string(&request) {
                    crate::host::request(&json);
                }
            }),
        });
        if let Some(warning) = warning {
            vm.set_status(warning);
        }
        MODEL.with(|slot| *slot.borrow_mut() = Some(vm.clone()));
        phone_ui::set_image_loader(Rc::new(|key| crate::host::load_image(&key)));
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
