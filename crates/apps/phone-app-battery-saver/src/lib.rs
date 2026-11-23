use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_app_support::settings::update;
use phone_controls::{flat, pivot, toggle};
use phone_domain::Route;
use phone_sdk::{
    common::{muted, page, scroll, text},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn battery_settings(vm: Rc<PhoneViewModel>) -> Element {
    let settings = vm.data().settings.clone();
    let environment = vm.environment();
    let percent = environment
        .battery_percent
        .map(|level| format!("{level}%"))
        .unwrap_or_else(|| "not available".into());
    let charging = environment
        .charging
        .map(|value| {
            if value {
                "charging"
            } else if environment.external_power == Some(true) {
                "connected to power"
            } else {
                "not charging"
            }
        })
        .unwrap_or("battery information unavailable");
    let save = vm.clone();
    let motion = vm.clone();
    rsx! {StackPanel{spacing:20.0,
        {muted("Remaining battery life",18.0)}{text(&percent,55.0)}{muted(charging,19.0)}
        {toggle("Battery Saver",settings.battery_saver,move |enabled|update(&save,|s|s.battery_saver=enabled))}
        if environment.power_saver==Some(true) {{muted("Your device is also saving power. Decorative motion and live tiles pause until that policy changes.",16.0)}}
        {muted("Pauses live-tile animation and decorative motion inside this app. Media playback and saved changes remain available.",17.0)}
        {flat("ease of access",move ||motion.navigate(Route::Setting("ease of access".into())))}
        {muted("Battery readings are shown only when your host supplies them. Your device controls charging and system power policies.",15.0)}
    }}
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let selected = ui.pivot.get().min(1);
    let usage = ui.pivot;
    let settings = ui.pivot;
    let environment = vm.environment();
    let level = environment
        .battery_percent
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "–".into());
    let body = if selected == 1 {
        rsx! {Border{margin:20.0,{battery_settings(vm.clone())}}}
    } else {
        rsx! {StackPanel{spacing:20.0,margin:20.0,
            {text(&level,72.0)}{text("remaining battery",24.0)}
            {muted(if environment.charging==Some(true){"Connected to power"}else if environment.battery_percent.is_some(){"Using your host's current battery reading"}else{"This host does not expose its battery level."},18.0)}
            {muted("Per-app battery usage is not available from this host. This page does not estimate another app's power consumption.",17.0)}
        }}
    };
    page(
        "battery saver",
        scroll(rsx! {StackPanel{
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0),
                {pivot("usage",selected==0,move ||usage.set(0))}{pivot("settings",selected==1,move ||settings.set(1))}}
            {body}
        }}),
        Element::Empty,
    )
}

mod registration;
pub use registration::registration;
