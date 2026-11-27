//! Retained Settings pivots with a separate finite scroller for each category.
use super::*;
use ferroui_controls::primitives::ScrollBarVisibility;
use phone_controls::{pivot_pages, PivotPage};

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    page(
        "settings",
        pivot_pages(
            "SettingsPivot",
            ui.pivot,
            vec![
                PivotPage::new("system", pane(vm.clone(), true)),
                PivotPage::new("applications", pane(vm, false)),
            ],
        ),
        Element::Empty,
    )
}

fn pane(vm: Rc<PhoneViewModel>, system: bool) -> Element {
    let settings = vm.data().settings.clone();
    let mut items = if system {
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
    if system
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
    if system && vm.platform_state().supports_device_snapshot() {
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
    rsx! { ScrollViewer {
        name:if system { "SettingsSystemScroll" } else { "SettingsApplicationsScroll" },
        horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
        vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
        StackPanel { for item in rows {{item}} }
    }}
}
