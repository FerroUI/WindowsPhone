//! Windows Phone Settings category navigation and shared mutation helpers.
mod appearance;
mod device;
mod hub;
mod integrations;
mod lock;
mod notification_apps;
mod photo_picker;
mod quick_actions;
mod region;
mod system;

use ferroui_view::prelude::*;
use phone_domain::{AppId, Route, Settings};
use phone_sdk::{
    common::{page, row, scroll, title},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Setting(name) = vm.route() {
        let native_control = device::control_for_page(&name).filter(|_| {
            vm.platform_state()
                .supports(phone_sdk::host::platform::HostFeature::HardwareControls)
        });
        let content = if let Some(control) = native_control {
            phone_sdk::device::control(vm.clone(), control)
        } else {
            match name.as_str() {
                "device services" => device::view(vm.clone()),
                "phone+messages" => device::communications(vm.clone()),
                "start+theme" => appearance::theme(vm.clone()),
                "ringtones+sounds" | "music+videos" => appearance::sounds(vm.clone(), ui.clone()),
                "notifications+actions" => quick_actions::notifications(vm.clone(), ui.clone()),
                "lock screen" => lock::view(vm.clone(), ui.clone()),
                "brightness" => appearance::display(vm.clone(), false),
                "ease of access" => appearance::display(vm.clone(), true),
                "date+time" => system::date_time(vm.clone()),
                "region" => region::view(vm.clone()),
                "storage sense" => phone_storage_support::storage(vm.clone()),
                "battery saver" => phone_app_battery_saver::battery_settings(vm.clone()),
                "backup" => system::backup(vm.clone()),
                "about" => system::about(vm.clone(), ui.clone()),
                "phone update" => system::phone_update(vm.clone()),
                "photos+camera" => integrations::photos(vm.clone()),
                "Internet Explorer" => integrations::browser(vm.clone()),
                "messaging" => integrations::messaging(vm.clone()),
                "maps" | "location" => integrations::location(vm.clone()),
                value if value.starts_with("notification:") => {
                    let app = AppId::ALL
                        .into_iter()
                        .find(|app| Some(app.label()) == value.strip_prefix("notification:"))
                        .unwrap_or(AppId::Messaging);
                    notification_apps::view(vm.clone(), app)
                }
                _ => system::radio(vm.clone(), &name),
            }
        };
        let heading = if name.starts_with("notification:") {
            "notifications+actions"
        } else {
            &name
        };
        return page(
            "settings",
            scroll(
                rsx! {StackPanel{spacing:12.0,{title(heading)}Border{margin:(20.0,0.0,20.0,24.0),{content}}}},
            ),
            Element::Empty,
        );
    }
    hub::view(vm, ui)
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

fn update(vm: &PhoneViewModel, change: impl FnOnce(&mut Settings)) {
    let mut settings = vm.data().settings.clone();
    change(&mut settings);
    vm.set_settings(settings);
}

mod registration;
pub use registration::registration;
