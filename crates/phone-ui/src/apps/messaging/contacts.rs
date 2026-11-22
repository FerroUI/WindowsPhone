use crate::{
    common::{empty, page, row, scroll, title},
    session::Session,
};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let query = ui.query.get().to_lowercase();
    let mut contacts = vm
        .data()
        .contacts
        .iter()
        .filter(|contact| !contact.phone.trim().is_empty())
        .filter(|contact| {
            contact.name.to_lowercase().contains(&query) || contact.phone.contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    contacts.sort_by_key(|contact| contact.name.to_lowercase());
    let mut body = stack_panel().child(title("choose a contact"));
    let manual_vm = vm.clone();
    let manual_ui = ui.clone();
    body = body.child(row("enter a phone number", "", move || {
        manual_ui.messages.contacts.set(false);
        super::compose(manual_vm.clone(), manual_ui.clone(), String::new());
    }));
    body=body.child(rsx!{TextBox{name:"RecipientSearch",class:"phone-input",text:ui.query.two_way(),placeholder_text:"search contacts",automation_properties_name:"Find message recipient",margin:(20.0,0.0,20.0,12.0)}});
    if contacts.is_empty() {
        body = body.child(empty(
            "No contacts",
            "Enter a number or add a phone number to a contact in People.",
        ));
    }
    for contact in contacts {
        let model = vm.clone();
        let state = ui.clone();
        let number = contact.phone.clone();
        body = body.child(row(&contact.name, &contact.phone, move || {
            state.messages.contacts.set(false);
            super::compose(model.clone(), state.clone(), number.clone());
        }));
    }
    let close = ui.clone();
    page(
        "new message",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Close, "cancel", move || {
                close.messages.contacts.set(false)
            })],
            false,
            vec![],
        ),
    )
}
