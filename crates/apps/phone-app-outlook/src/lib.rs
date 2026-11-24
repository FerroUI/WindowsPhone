//! Local Outlook mailbox and persistent draft workflows. Sending is an explicit
//! handoff to the host mail app, without claiming delivery or mutating sent mail.
mod composer;
mod detail;
mod draft_list;
mod drafts;
mod list;
mod state;
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_sdk::session::Session;
use phone_state::PhoneViewModel;
pub use state::MailState;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui.state::<composer::State>().active.get() {
        return composer::view(vm, ui);
    }
    if let Route::Mail(id) = vm.route() {
        let mail = vm.data().mail(id).cloned();
        if let Some(mail) = mail {
            return detail::view(vm, ui, mail);
        }
    }
    list::view(vm, ui)
}

fn compose(_: Rc<PhoneViewModel>, ui: Rc<Session>, to: String, subject: String, body: String) {
    composer::begin(&ui, drafts::Fields { to, subject, body }, None);
}

mod registration;
pub use registration::registration;
