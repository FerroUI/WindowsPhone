//! Action Center policy and saved preferences for future per-app alert delivery.
use super::update;
use ferroui_view::prelude::*;
use phone_controls::toggle;
use phone_domain::{AppId, NotificationPreference};
use phone_sdk::common::{muted, text};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>, app: AppId) -> Element {
    let prefs = vm.data().settings.notification_preference(app);
    let center = vm.clone();
    let banners = vm.clone();
    let sound = vm.clone();
    let vibrate = vm.clone();
    rsx! {StackPanel{spacing:20.0,
        {text(app.label(),31.0)}
        {toggle("Show in action center",prefs.action_center,move |enabled|set(&center,app,|p|p.action_center=enabled))}
        {muted("Controls which notifications appear in this app's action center.",16.0)}
        {toggle("Show notification banners",prefs.banners,move |enabled|set(&banners,app,|p|p.banners=enabled))}
        {toggle("Play a sound",prefs.sound,move |enabled|set(&sound,app,|p|p.sound=enabled))}
        {toggle("Vibrate",prefs.vibrate,move |enabled|set(&vibrate,app,|p|p.vibrate=enabled))}
        {muted("Calendar and alarm banners can also appear on your device when notification permission is allowed. Other apps keep these preferences for local notifications; your host controls device delivery.",16.0)}
    }}
}

fn set(vm: &PhoneViewModel, app: AppId, change: impl FnOnce(&mut NotificationPreference)) {
    update(vm, |s| {
        change(s.app_notifications.entry(app.label().into()).or_default())
    });
}

pub(super) fn summary(prefs: NotificationPreference) -> String {
    let mut enabled = Vec::new();
    if prefs.action_center {
        enabled.push("action center");
    }
    if prefs.banners {
        enabled.push("banners");
    }
    if prefs.sound {
        enabled.push("sound");
    }
    if prefs.vibrate {
        enabled.push("vibrate");
    }
    if enabled.is_empty() {
        "off".into()
    } else {
        enabled.join(", ")
    }
}
