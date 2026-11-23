use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Messaging, crate::view)
        .with_launch_path(|path| {
            if path == "compose" {
                return Ok(serde_json::json!({"kind":"compose"}));
            }
            let id = phone_sdk::device::decode_route_value(
                path.strip_prefix("thread/")
                    .ok_or("Unknown messaging route.")?,
                1024,
            )?;
            phone_sdk::host::platform::DeviceRequest::Messages {
                thread_id: id.clone(),
                limit: 200,
            }
            .validate()?;
            Ok(serde_json::json!({"kind":"thread","id":id}))
        })
        .with_activation(
            |vm, ui, value| match value.get("kind").and_then(|v| v.as_str()) {
                Some("compose") => {
                    let state = ui.state::<crate::device::State>();
                    state.local.set(false);
                    state.composing.set(true);
                    crate::device::tick(vm, ui);
                    Ok(())
                }
                Some("thread") => {
                    let id = value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or("Message thread is missing.")?
                        .to_owned();
                    let request = phone_sdk::host::platform::DeviceRequest::Messages {
                        thread_id: id.clone(),
                        limit: 200,
                    };
                    request.validate()?;
                    let state = ui.state::<crate::device::State>();
                    state.local.set(false);
                    state.composing.set(false);
                    state.thread.set(Some(id));
                    vm.request_device(request);
                    Ok(())
                }
                _ => Err("Unknown messaging activation.".into()),
            },
        )
        .with_lifecycle(|cx, event| {
            if matches!(
                event,
                phone_sdk::AppLifecycle::Activated | phone_sdk::AppLifecycle::Resumed
            ) {
                crate::device::refresh(&cx.vm, &cx.session);
                crate::device::tick(&cx.vm, &cx.session);
            }
        })
        .with_tick(|vm, ui, _| crate::device::tick(vm, ui))
        .with_back(back)
        .with_route_changed(|_, ui, _, _| ui.state::<crate::MessageState>().reset())
        .with_deactivate(|ui| ui.state::<crate::MessageState>().reset())
        .with_reset(|ui| {
            let state = ui.state::<crate::MessageState>();
            state.reset();
            state.drafts.set(Default::default());
            crate::device::reset(ui);
        })
}

fn back(_: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    if crate::device::back(ui) {
        return true;
    }
    let dismissed = ui.state::<crate::MessageState>().dismiss();
    if dismissed {
        ui.query.set(String::new());
    }
    dismissed
}
