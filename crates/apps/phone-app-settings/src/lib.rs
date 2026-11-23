//! Windows Phone Settings category navigation and shared mutation helpers.
mod appearance;
mod device;
mod integrations;
mod lock;
mod notification_apps;
mod photo_picker;
mod quick_actions;
mod region;
mod system;

use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::pivot;
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
    let p = ui.pivot.get().min(1);
    let system = ui.pivot;
    let applications = ui.pivot;
    let settings = vm.data().settings.clone();
    let mut items = if p == 0 {
        vec![
            (
                "start+theme",
                format!(
                    "{}, {}",
                    if settings.dark { "dark" } else { "light" },
                    settings.accent_name()
                ),
            ),
            (
                "ringtones+sounds",
                if settings.sounds {
                    "sounds on".into()
                } else {
                    "silent".into()
                },
            ),
            (
                "notifications+actions",
                if settings.notifications {
                    "on".into()
                } else {
                    "off".into()
                },
            ),
            (
                "lock screen",
                if settings.lock_timeout == 0 {
                    "never".into()
                } else {
                    format!("{} seconds", settings.lock_timeout)
                },
            ),
            ("Wi-Fi", on_off(settings.wi_fi).into()),
            ("Bluetooth", on_off(settings.bluetooth).into()),
            ("flight mode", on_off(settings.airplane).into()),
            (
                "screen rotation",
                if settings.rotation_lock {
                    "locked".into()
                } else {
                    "unlocked".into()
                },
            ),
            (
                "brightness",
                format!("{}%", (settings.brightness * 100.0).round()),
            ),
            ("battery saver", on_off(settings.battery_saver).into()),
            ("storage sense", "phone storage".into()),
            ("location", on_off(settings.location).into()),
            (
                "date+time",
                if settings.time_24h {
                    "24-hour clock".into()
                } else {
                    "12-hour clock".into()
                },
            ),
            (
                "ease of access",
                format!("text size {}%", (settings.text_scale * 100.0).round()),
            ),
            ("region", settings.region.clone()),
            ("backup", "apps, settings and local content".into()),
            ("about", settings.phone_name.clone()),
            ("phone update", "check for app updates".into()),
        ]
    } else {
        vec![
            ("Internet Explorer", "favorites and browsing".into()),
            ("maps", on_off(settings.location).into()),
            ("messaging", "notifications and messages".into()),
            ("music+videos", "volume and playback".into()),
            ("photos+camera", "photos and camera".into()),
        ]
    };
    if p == 0
        && vm
            .platform_state()
            .supports(phone_sdk::host::platform::HostFeature::HardwareControls)
    {
        for (name, description) in &mut items {
            if let Some(control) = device::control_for_page(name) {
                *description = phone_sdk::device::control_summary(&vm, control);
            }
        }
        for control in phone_sdk::host::platform::DeviceControl::ALL {
            let state = vm.device_state();
            if state
                .snapshot
                .control(control)
                .is_some_and(|v| v.access != phone_sdk::host::platform::ControlAccess::Unavailable)
                && !items
                    .iter()
                    .any(|(name, _)| device::control_for_page(name) == Some(control))
                && device::control_for_page(control.label()) == Some(control)
            {
                items.push((
                    control.label(),
                    phone_sdk::device::control_summary(&vm, control),
                ));
            }
        }
        items.push(("phone+messages", "calling, messaging and contacts".into()));
    }
    if p == 0 && vm.platform_state().supports_device_snapshot() {
        items.push((
            "device services",
            "roles, permissions and system settings".into(),
        ));
    }
    let rows = items
        .into_iter()
        .map(|(name, description)| {
            let model = vm.clone();
            row(name, &description, move || {
                model.navigate(Route::Setting(name.into()))
            })
        })
        .collect::<Vec<_>>();
    let body = rsx! {StackPanel{
        StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0,0.0,10.0),
            {pivot("system",p==0,move ||system.set(0))}
            {pivot("applications",p==1,move ||applications.set(1))}
        }
        for item in rows {{item}}
    }};
    page("settings", scroll(body), Element::Empty)
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
