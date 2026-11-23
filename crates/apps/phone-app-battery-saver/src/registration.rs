use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::BatterySaver, crate::view)
        .with_pivot_count(|_| 2)
        .with_lifecycle(|cx, event| {
            if matches!(
                event,
                phone_sdk::AppLifecycle::Activated | phone_sdk::AppLifecycle::Resumed
            ) && cx
                .vm
                .platform_state()
                .supports(phone_sdk::host::platform::HostFeature::HardwareControls)
            {
                cx.vm.refresh_device();
            }
        })
}
