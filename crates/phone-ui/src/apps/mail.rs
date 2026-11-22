//! Local Outlook mailbox workflows; composition is handed to the host mail app.
mod detail;
mod list;
mod state;
use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_state::{HostRequest, PhoneViewModel};
pub use state::MailState;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Mail(id) = vm.route() {
        let mail = vm.data().mail(id).cloned();
        if let Some(mail) = mail {
            return detail::view(vm, ui, mail);
        }
    }
    list::view(vm, ui)
}

fn compose(vm: Rc<PhoneViewModel>, ui: Rc<Session>, to: String, subject: String, body: String) {
    let error = ui.form_error;
    ui.form_with_action(
        "new email",
        vec![("to", to), ("subject", subject), ("body", body)],
        "open email",
        Rc::new(move |values| {
            let to = values
                .first()
                .cloned()
                .unwrap_or_default()
                .trim()
                .to_owned();
            if to.is_empty() || !to.contains('@') || to.contains(['\r', '\n']) {
                error.set("Enter an email address.".into());
                return;
            }
            vm.request(HostRequest::Email {
                to,
                subject: values.get(1).cloned().unwrap_or_default(),
                body: values.get(2).cloned().unwrap_or_default(),
            });
        }),
    );
}
