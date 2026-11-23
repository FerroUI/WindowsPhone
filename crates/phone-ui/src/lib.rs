//! Default application composition. The shell and SDK are independently reusable.
use ferroui_base::Ref;
use ferroui_controls::Control;
pub use phone_sdk::{AppContext, AppManifest, AppRegistration, AppRegistry};
use phone_state::PhoneViewModel;
pub use phone_system::*;
use std::rc::Rc;

pub fn built_in_apps() -> Vec<AppRegistration> {
    vec![
        phone_app_alarms::registration(),
        phone_app_battery_saver::registration(),
        phone_app_browser::registration(),
        phone_app_calculator::registration(),
        phone_app_calendar::registration(),
        phone_app_camera::registration(),
        phone_app_cortana::registration(),
        phone_app_data_sense::registration(),
        phone_app_files::registration(),
        phone_app_games::registration(),
        phone_app_maps::registration(),
        phone_app_messaging::registration(),
        phone_app_minesweeper::registration(),
        phone_app_music::registration(),
        phone_app_office::registration(),
        phone_app_onenote::registration(),
        phone_app_outlook::registration(),
        phone_app_people::registration(),
        phone_app_phone::registration(),
        phone_app_photos::registration(),
        phone_app_settings::registration(),
        phone_app_store::registration(),
        phone_app_tic_tac_toe::registration(),
        phone_app_video::registration(),
        phone_app_weather::registration(),
    ]
}
pub fn create_view(vm: Rc<PhoneViewModel>) -> Ref<Control> {
    create_view_with_apps(vm, Vec::new()).expect("default app registry")
}
/// Compose built-ins and additional independently compiled SDK applications.
pub fn create_view_with_apps(
    vm: Rc<PhoneViewModel>,
    additional: Vec<AppRegistration>,
) -> Result<Ref<Control>, String> {
    let mut apps = built_in_apps();
    apps.extend(additional);
    let registry = Rc::new(AppRegistry::new(apps)?);
    Ok(phone_system::create_view(vm, registry))
}
