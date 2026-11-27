use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat};
use phone_domain::{AppId, Contact, Route};
use phone_sdk::{
    common::{muted, page, scroll, text},
    session::Session,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;
mod device;
mod hub;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Contact(id) = vm.route() {
        let contact = vm.data().contact(id).cloned();
        if let Some(contact) = contact {
            return detail(vm, ui, contact);
        }
    }
    hub::view(vm, ui)
}

fn detail(vm: Rc<PhoneViewModel>, ui: Rc<Session>, contact: Contact) -> Element {
    let call = vm.clone();
    let number = contact.phone.clone();
    let name = contact.name.clone();
    let message = vm.clone();
    let message_ui = ui.clone();
    let message_number = contact.phone.clone();
    let email = vm.clone();
    let address = contact.email.clone();
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let editable = contact.clone();
    let favorite = vm.clone();
    let pin = vm.clone();
    let id = contact.id;
    let remove = vm.clone();
    let confirm = ui.clone();
    let menu = ui.menu;
    let mut more = phone_app_support::pim::contacts(vm.clone(), ui.clone(), Some(id));
    more.push(flat("pin to start", move || {
        pin.pin_contact(id);
        pin.home();
    }));
    more.push(flat("delete contact", move || {
        let model = remove.clone();
        confirm.confirm(
            "delete this contact?",
            Rc::new(move || {
                model.delete_contact(id);
                model.navigate(Route::App(AppId::People));
            }),
        );
    }));
    let body = rsx! {StackPanel{margin:20.0,spacing:18.0,
        Border{background:contact.color.as_str(),width:100.0,height:100.0,horizontal_alignment:H::Left,
            TextBlock{text:contact.initials(),font_size:42.0,foreground:"White",horizontal_alignment:H::Center,vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}}
        {text(&contact.name,42.0)}
        if !contact.company.is_empty(){{muted(&contact.company,23.0)}}
        if !contact.phone.is_empty(){
            {flat(&format!("call mobile\n{}",contact.phone),move ||call.record_call(name.clone(),number.clone()))}
            {flat("text",move ||phone_app_messaging::compose(message.clone(),message_ui.clone(),message_number.clone()))}
        }
        if !contact.email.is_empty(){
            {flat(&format!("send email\n{}",contact.email),move ||email.request(HostRequest::Email{to:address.clone(),subject:String::new(),body:String::new()}))}
        }
    }};
    let bar = app_bar(
        vec![
            action(Icon::Edit, "edit", move || {
                edit(edit_vm.clone(), edit_ui.clone(), editable.clone())
            }),
            action(
                if contact.favorite {
                    Icon::Heart
                } else {
                    Icon::Star
                },
                if contact.favorite {
                    "unfavorite"
                } else {
                    "favorite"
                },
                move || favorite.toggle_favorite_contact(id),
            ),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        more,
    );
    page("people", scroll(body), bar)
}

fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, contact: Contact) {
    let error = ui.form_error;
    let old = contact.clone();
    ui.form(
        if contact.id == 0 {
            "new contact"
        } else {
            "edit contact"
        },
        vec![
            ("name", contact.name),
            ("phone", contact.phone),
            ("email", contact.email),
            ("company", contact.company),
        ],
        Rc::new(move |values: Vec<String>| {
            let field = |index: usize| {
                values
                    .get(index)
                    .cloned()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            };
            let name = field(0);
            if name.is_empty() {
                error.set("Enter a contact name.".into());
                return;
            }
            if name.chars().count() > 256 {
                error.set("Keep the contact name under 257 characters.".into());
                return;
            }
            vm.save_contact(Contact {
                id: old.id,
                name,
                phone: field(1),
                email: field(2),
                company: field(3),
                favorite: old.favorite,
                color: old.color.clone(),
            });
        }),
    );
}

mod registration;
pub use registration::registration;
