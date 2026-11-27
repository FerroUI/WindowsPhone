use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Settings, crate::view)
        .with_route_pivot_count(|route, _| {
            if matches!(
                route,
                phone_domain::Route::App(phone_domain::AppId::Settings)
            ) {
                2
            } else {
                0
            }
        })
        .with_launch_path(|path| {
            let page = match path {
                "device" => "device services".to_owned(),
                "communications" => "phone+messages".to_owned(),
                _ => {
                    let key = path
                        .strip_prefix("control/")
                        .ok_or("Unknown Settings route.")?;
                    phone_sdk::host::platform::DeviceControl::from_key(key)
                        .ok_or("Unknown device control.")?;
                    format!("device/{key}")
                }
            };
            Ok(serde_json::json!({"page":page}))
        })
        .with_activation(|vm, _, value| {
            let page = value
                .get("page")
                .and_then(|v| v.as_str())
                .ok_or("A Settings page is required.")?;
            if !matches!(page, "device services" | "phone+messages")
                && crate::device::control_for_page(page).is_none()
            {
                return Err("Unknown Settings page.".into());
            }
            vm.navigate(phone_domain::Route::Setting(page.into()));
            Ok(())
        })
        .with_lifecycle(|cx, event| {
            if matches!(
                event,
                phone_sdk::AppLifecycle::Activated | phone_sdk::AppLifecycle::Resumed
            ) && cx.vm.platform_state().supports_device_snapshot()
            {
                cx.vm.refresh_device();
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_domain::{AppId, Route};
    use phone_sdk::{AppRegistry, session::Session};
    use std::rc::Rc;

    #[test]
    fn only_the_settings_hub_advertises_horizontal_pivots() {
        let registry = Rc::new(AppRegistry::new(vec![registration()]).unwrap());
        let ui = Session::new(0, registry.clone());
        ui.pivot.set(1);
        assert_eq!(registry.pivot_count(&Route::App(AppId::Settings), &ui), 2);
        assert!(registry.can_swipe(&Route::App(AppId::Settings), &ui));
        for name in [
            "brightness",
            "ringtones+sounds",
            "device services",
            "notification:Phone",
        ] {
            let detail = Route::Setting(name.into());
            assert_eq!(registry.pivot_count(&detail, &ui), 0, "{name}");
            assert!(!registry.can_swipe(&detail, &ui), "{name}");
        }
        assert_eq!(ui.pivot.get_untracked(), 1);
    }
}
