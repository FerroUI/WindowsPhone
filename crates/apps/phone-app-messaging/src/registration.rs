use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Messaging, crate::view)
        .with_back(back)
        .with_route_changed(|_, ui, _, _| ui.state::<crate::MessageState>().reset())
        .with_deactivate(|ui| ui.state::<crate::MessageState>().reset())
        .with_reset(|ui| {
            let state = ui.state::<crate::MessageState>();
            state.reset();
            state.drafts.set(Default::default());
        })
}

fn back(_: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    let dismissed = ui.state::<crate::MessageState>().dismiss();
    if dismissed {
        ui.query.set(String::new());
    }
    dismissed
}
