use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Outlook, crate::view)
        .with_pivot_count(|_| 3)
        .with_back(back)
        .with_route_changed(|_, ui, _, _| ui.state::<crate::MailState>().reset())
        .with_deactivate(|ui| ui.state::<crate::MailState>().reset())
        .with_reset(|ui| {
            let state = ui.state::<crate::MailState>();
            state.reset();
            state.folder.set("inbox".into());
        })
}

fn back(_: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    let dismissed = ui.state::<crate::MailState>().dismiss();
    if dismissed {
        ui.query.set(String::new());
    }
    dismissed
}
