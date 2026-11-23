use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::People, crate::view)
        .with_pivot_count(|ui| {
            if ui.state::<crate::device::State>().available.get() {
                3
            } else {
                2
            }
        })
        .with_launch_path(|path| {
            if path == "device" {
                Ok(serde_json::json!({"kind":"device"}))
            } else {
                Err("Unknown People route.".into())
            }
        })
        .with_activation(|vm, ui, value| {
            if value.get("kind").and_then(|v| v.as_str()) != Some("device") {
                return Err("Unknown People activation.".into());
            }
            if !vm
                .platform_state()
                .supports(phone_sdk::host::platform::HostFeature::Contacts)
            {
                return Err("Device contacts are unavailable on this host.".into());
            }
            ui.pivot.set(2);
            crate::device::refresh(vm, ui);
            Ok(())
        })
        .with_lifecycle(|cx, event| {
            if matches!(
                event,
                phone_sdk::AppLifecycle::Activated | phone_sdk::AppLifecycle::Resumed
            ) {
                crate::device::refresh(&cx.vm, &cx.session);
            }
        })
        .with_tick(|vm, ui, _| crate::device::tick(vm, ui))
        .with_back(|_, ui| {
            let state = ui.state::<crate::device::State>();
            if state.selected.get_untracked().is_some() {
                state.selected.set(None);
                true
            } else {
                false
            }
        })
        .with_reset(|ui| {
            let state = ui.state::<crate::device::State>();
            state.selected.set(None);
            state.query.set(String::new());
        })
}
