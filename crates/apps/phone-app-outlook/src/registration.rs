use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Outlook, crate::view)
        .with_pivot_count(|ui| {
            let mail = ui.state::<crate::MailState>();
            if ui.state::<crate::composer::State>().active.get_untracked()
                || mail.folders.get_untracked()
                || mail.saved_drafts.get_untracked()
            {
                0
            } else {
                3
            }
        })
        .with_back(back)
        .with_route_changed(|_, ui, _, _| ui.state::<crate::MailState>().reset())
        .with_deactivate(|ui| ui.state::<crate::MailState>().reset())
        .with_reset(|ui| {
            let state = ui.state::<crate::MailState>();
            state.reset();
            state.folder.set("inbox".into());
            state.saved_drafts.set(false);
            ui.state::<crate::composer::State>().close();
        })
}

fn back(_: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    if crate::composer::back(ui) {
        return true;
    }
    let dismissed = ui.state::<crate::MailState>().dismiss();
    if dismissed {
        ui.query.set(String::new());
    }
    dismissed
}
