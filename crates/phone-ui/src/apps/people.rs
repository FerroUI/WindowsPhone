use crate::{
    common::{empty, muted, page, row, scroll, text},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_controls::{Control, primitives::ScrollBarVisibility};
use ferroui_dynamic_data::IObservableCache;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, pivot};
use phone_domain::{AppId, Contact, Route};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Contact(id) = vm.route() {
        let contact = vm.data().contact(id).cloned();
        if let Some(contact) = contact {
            return detail(vm, ui, contact);
        }
    }
    let mode = ui.pivot.get().min(1);
    let query = ui.query.get().to_lowercase();
    let mut contacts = vm
        .contacts
        .items()
        .into_iter()
        .filter(|contact| {
            (!query.is_empty()
                && format!(
                    "{} {} {} {}",
                    contact.name, contact.phone, contact.email, contact.company
                )
                .to_lowercase()
                .contains(&query))
                || query.is_empty()
        })
        .filter(|contact| mode == 0 || contact.favorite)
        .collect::<Vec<_>>();
    contacts.sort_by_key(|contact| (group_letter(&contact.name), contact.name.to_lowercase()));
    let mut list = stack_panel();
    let mut last = None;
    let mut available = Vec::new();
    for contact in contacts.iter() {
        let letter = group_letter(&contact.name);
        if last != Some(letter) {
            last = Some(letter);
            available.push(letter);
            let show = ui.alphabet;
            list=list.child(rsx!{Button{name:format!("PeopleLetter{letter}"),class:"phone-flat",margin:(20.0,14.0,0.0,4.0),padding:0.0,
                width:38.0,height:38.0,horizontal_alignment:H::Left,automation_properties_name:format!("Jump from letter {letter}"),
                on_click:move |_|show.set(true),
                Border{border_brush:vm.data().settings.accent.as_str(),border_thickness:2.0,width:34.0,height:34.0,
                    TextBlock{text:letter.to_string(),font_size:23.0,foreground:vm.data().settings.accent.as_str(),horizontal_alignment:H::Center}}
            }});
        }
        let model = vm.clone();
        let id = contact.id;
        list = list.child(row(&contact.name, &contact.company, move || {
            model.navigate(Route::Contact(id))
        }));
    }
    if contacts.is_empty() {
        list = list.child(empty(
            "No contacts",
            if query.is_empty() {
                "Tap new to add someone."
            } else {
                "Try a different name or number."
            },
        ));
    }
    let all = ui.pivot;
    let favorites = ui.pivot;
    let overlay = ui.alphabet.get();
    let body = rsx! {Grid{name:"PeopleRoot",row_definitions:"Auto,*",
        StackPanel{margin:(20.0,0.0,20.0,10.0),spacing:8.0,
            StackPanel{orientation:Orientation::Horizontal,
                {pivot("all",mode==0,move ||all.set(0))}
                {pivot("favorites",mode==1,move ||favorites.set(1))}
            }
            TextBox{class:"phone-input",text:ui.query.two_way(),placeholder_text:"search contacts",automation_properties_name:"Search contacts"}
        }
        ScrollViewer{key:"people-list",Grid.row:1,horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,{list}}
        if overlay {Border{Grid.row_span:2,background:vm.data().settings.background(),{alphabet(ui.clone(),available)}}}
    }};
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let menu = ui.menu;
    let jump = ui.alphabet;
    let mut more = super::pim::contacts(vm.clone(), ui.clone(), None);
    more.push(flat("show all contacts", move || {
        ui.pivot.set(0);
        ui.query.set(String::new());
        ui.menu.set(false);
    }));
    let expanded = menu.get();
    let bar = app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                edit(add_vm.clone(), add_ui.clone(), Contact::default())
            }),
            action(Icon::List, "jump", move || jump.set(true)),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        expanded,
        more,
    );
    page("people", body, bar)
}

fn alphabet(ui: Rc<Session>, available: Vec<char>) -> Element {
    let mut letters = grid()
        .column_definitions("*,*,*,*")
        .row_definitions("60,60,60,60,60,60,60")
        .row_spacing(8.0)
        .column_spacing(8.0)
        .margin(20.0);
    for (index, letter) in std::iter::once('#').chain('a'..='z').enumerate() {
        let close = ui.alphabet;
        let enabled = available.contains(&letter);
        letters=letters.child(rsx!{Button{Grid.row:(index/4)as i32,Grid.column:(index%4)as i32,class:"phone-button",
            content:letter.to_string(),font_size:30.0,is_enabled:enabled,automation_properties_name:format!("Jump to {letter}"),
            on_click:move |event|{
                let root=event.source().and_then(|source|source.cast::<Control>()).and_then(|source|source.find_ancestor_of_type_where::<Control>(true,|control|control.name().as_deref()==Some("PeopleRoot")));
                if let Some(root)=root {
                    let name=format!("PeopleLetter{letter}");
                    if let Some(target)=root.find_descendant_of_type_where::<Control>(false,|control|control.name().as_deref()==Some(name.as_str())){target.bring_into_view();}
                }
                close.set(false);event.set_handled(true);
            }
        }});
    }
    scroll(letters.into())
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
    let mut more = super::pim::contacts(vm.clone(), ui.clone(), Some(id));
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
            {flat("text",move ||super::messaging::compose(message.clone(),message_ui.clone(),message_number.clone()))}
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

fn group_letter(name: &str) -> char {
    name.trim_start()
        .chars()
        .next()
        .filter(char::is_ascii_alphabetic)
        .map(|letter| letter.to_ascii_lowercase())
        .unwrap_or('#')
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
