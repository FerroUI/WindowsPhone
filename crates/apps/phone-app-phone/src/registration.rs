use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Phone, crate::view)
        .with_launch_path(|path| {
            if path == "calls" {
                return Ok(serde_json::json!({"kind":"calls"}));
            }
            let number = phone_sdk::device::decode_route_value(
                path.strip_prefix("dial/").ok_or("Unknown phone route.")?,
                128,
            )?;
            if !phone_sdk::host::platform::phone_number(&number) {
                return Err("Invalid phone number.".into());
            }
            Ok(serde_json::json!({"kind":"dial","number":number}))
        })
        .with_activation(
            |vm, ui, value| match value.get("kind").and_then(|v| v.as_str()) {
                Some("calls") => {
                    crate::device::refresh(vm);
                    Ok(())
                }
                Some("dial") => {
                    let number = value
                        .get("number")
                        .and_then(|v| v.as_str())
                        .ok_or("A phone number is required.")?;
                    if !phone_sdk::host::platform::phone_number(number) {
                        return Err("Invalid phone number.".into());
                    }
                    ui.pivot.set(2);
                    ui.state::<crate::PhoneSession>().dial.set(number.into());
                    Ok(())
                }
                _ => Err("Unknown phone activation.".into()),
            },
        )
        .with_lifecycle(|cx, event| {
            if matches!(
                event,
                phone_sdk::AppLifecycle::Activated | phone_sdk::AppLifecycle::Resumed
            ) {
                crate::device::refresh(&cx.vm);
            }
        })
        .with_tick(|vm, ui, now| {
            let state = ui.state::<crate::PhoneSession>();
            let observed = vm.device_state();
            if !observed.calls.is_empty() {
                state.clock.set(now);
            }
            let readable = (
                observed
                    .snapshot
                    .supports(phone_sdk::host::platform::DeviceOperation::ActiveCalls),
                observed
                    .snapshot
                    .supports(phone_sdk::host::platform::DeviceOperation::CallHistory),
            );
            if readable != state.readable.replace(readable) && (readable.0 || readable.1) {
                crate::device::refresh(vm);
            }
        })
        .with_pivot_count(|ui| if ui.pivot.get_untracked() < 2 { 2 } else { 0 })
        .with_reset(|ui| {
            let state = ui.state::<crate::PhoneSession>();
            state.dial.set(String::new());
            state.account.set(None);
            state.clock.set(0);
            state.readable.set((false, false));
        })
}
