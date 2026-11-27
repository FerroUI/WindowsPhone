use phone_sdk::AppRegistration;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::People, crate::view)
        .with_route_pivot_count(|route, ui| {
            if !matches!(route, phone_domain::Route::App(phone_domain::AppId::People)) {
                return 0;
            }
            let state = ui.state::<crate::device::State>();
            if state.available.get()
                && ui.pivot.get_untracked() == 2
                && state.selected.get_untracked().is_some()
            {
                0
            } else if state.available.get() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use phone_domain::{AppId, PhoneData, Route};
    use phone_sdk::{AppRegistry, session::Session};
    use phone_state::PhoneViewModel;
    use std::rc::Rc;

    #[test]
    fn local_and_device_contact_details_cannot_switch_hidden_people_pivots() {
        let registry = Rc::new(AppRegistry::new(vec![registration()]).unwrap());
        let ui = Rc::new(Session::new(0, registry.clone()));
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        vm.navigate(Route::App(AppId::People));
        let root = vm.route();
        assert_eq!(registry.pivot_count(&root, &ui), 2);
        assert!(!registry.can_swipe(&Route::Contact(42), &ui));
        let state = ui.state::<crate::device::State>();
        state.available.set(true);
        assert_eq!(registry.pivot_count(&root, &ui), 3);
        ui.pivot.set(2);
        state.selected.set(Some("native-contact-42".into()));
        assert_eq!(registry.pivot_count(&root, &ui), 0);
        assert!(!registry.can_swipe(&root, &ui));
        assert!(registry.back(&vm, &ui));
        assert!(state.selected.get_untracked().is_none());
        assert_eq!(ui.pivot.get_untracked(), 2);
        assert_eq!(registry.pivot_count(&root, &ui), 3);
        assert!(!registry.back(&vm, &ui));
        state.available.set(false);
        assert_eq!(registry.pivot_count(&root, &ui), 2);
    }
}
