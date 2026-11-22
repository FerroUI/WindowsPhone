//! Windows Phone Settings category navigation and shared mutation helpers.
mod appearance;
mod integrations;
mod quick_actions;
mod system;

use crate::{
    common::{page, row, scroll, title},
    session::Session,
};
use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::pivot;
use phone_domain::{Route, Settings};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Setting(name) = vm.route() {
        let content = match name.as_str() {
            "start+theme" => appearance::theme(vm.clone()),
            "ringtones+sounds" | "music+videos" => appearance::sounds(vm.clone()),
            "notifications+actions" => quick_actions::notifications(vm.clone(), ui.clone()),
            "lock screen" => system::lock(vm.clone(), ui.clone()),
            "brightness" => appearance::display(vm.clone(), false),
            "ease of access" => appearance::display(vm.clone(), true),
            "date+time" => system::date_time(vm.clone()),
            "backup" => system::backup(vm.clone()),
            "about" => system::about(vm.clone(), ui.clone()),
            "photos+camera" => integrations::photos(vm.clone()),
            "Internet Explorer" => integrations::browser(vm.clone()),
            "messaging" => integrations::messaging(vm.clone()),
            "maps" => integrations::location(vm.clone()),
            _ => system::radio(vm.clone(), &name),
        };
        return page(
            "settings",
            scroll(
                rsx! {StackPanel{spacing:12.0,{title(&name)}Border{margin:(20.0,0.0,20.0,24.0),{content}}}},
            ),
            Element::Empty,
        );
    }
    let p = ui.pivot.get().min(1);
    let system = ui.pivot;
    let applications = ui.pivot;
    let settings = vm.data().settings.clone();
    let items = if p == 0 {
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
            ("backup", "apps, settings and local content".into()),
            ("about", settings.phone_name.clone()),
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
    if value { "on" } else { "off" }
}

fn update(vm: &PhoneViewModel, change: impl FnOnce(&mut Settings)) {
    let mut settings = vm.data().settings.clone();
    change(&mut settings);
    vm.set_settings(settings);
}
