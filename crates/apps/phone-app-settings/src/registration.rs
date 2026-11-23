use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Settings, crate::view)
        .with_pivot_count(|_| 2)
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
