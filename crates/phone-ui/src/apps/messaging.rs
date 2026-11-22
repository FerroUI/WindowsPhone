//! Local message threads and host SMS composition with explicit draft status.
mod bubbles;
mod contacts;
mod list;
mod state;
mod thread;
use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_state::PhoneViewModel;
pub use state::MessageState;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui.messages.contacts.get() {
        return contacts::view(vm, ui);
    }
    if let Route::Conversation(id) = vm.route() {
        let conversation = vm.data().conversation(id).cloned();
        if let Some(conversation) = conversation {
            return thread::view(vm, ui, conversation);
        }
    }
    list::view(vm, ui)
}

pub fn compose(vm: Rc<PhoneViewModel>, ui: Rc<Session>, recipient: String) {
    compose_body(vm, ui, recipient, String::new());
}

pub(super) fn compose_body(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    recipient: String,
    body: String,
) {
    let error = ui.form_error;
    ui.form_with_action(
        "new message",
        vec![("to", recipient), ("message", body)],
        "open message",
        Rc::new(move |values| {
            let to = values
                .first()
                .cloned()
                .unwrap_or_default()
                .trim()
                .to_owned();
            let body = values.get(1).cloned().unwrap_or_default();
            if to.is_empty() {
                error.set("Enter a recipient.".into());
                return;
            }
            if body.trim().is_empty() {
                error.set("Type a message.".into());
                return;
            }
            let id = vm.create_conversation(to);
            if id == 0 {
                error.set(vm.status());
                return;
            }
            vm.navigate(Route::Conversation(id));
            vm.send_message(id, body);
        }),
    );
}
