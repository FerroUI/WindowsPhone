use crate::{
    common::{empty, muted, page, row, scroll, text},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mode = ui.pivot.get().min(2);
    let pivots = ["history", "speed dial"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let state = ui.pivot;
            pivot(label, mode == index, move || state.set(index))
        })
        .collect::<Vec<_>>();
    let body = if mode == 2 {
        dialpad(vm.clone(), ui.clone())
    } else {
        let mut content=stack_panel().child(rsx!{StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0,0.0,14.0),for item in pivots {{item}}}});
        if mode == 0 {
            let calls = vm.data().call_history.clone();
            if calls.is_empty() {
                content = content.child(empty(
                    "No calls yet",
                    "Use the keypad or call someone in People.",
                ));
            }
            for call in calls {
                let model = vm.clone();
                let name = call.name.clone();
                let number = call.number.clone();
                let time = phone_domain::time_text(call.timestamp, vm.data().settings.time_24h);
                let subtitle = format!(
                    "{} · {}",
                    if call.missed {
                        "missed call"
                    } else if call.incoming {
                        "incoming"
                    } else {
                        "dialed"
                    },
                    time
                );
                content = content.child(row(
                    if call.name.is_empty() {
                        &call.number
                    } else {
                        &call.name
                    },
                    &subtitle,
                    move || model.record_call(name.clone(), number.clone()),
                ));
            }
        } else {
            let people = vm
                .data()
                .contacts
                .iter()
                .filter(|contact| contact.favorite)
                .cloned()
                .collect::<Vec<_>>();
            if people.is_empty() {
                content = content.child(empty(
                    "Keep your favorites close",
                    "Open People and add a contact to your favorites.",
                ));
            }
            for person in people {
                let model = vm.clone();
                let id = person.id;
                content = content.child(row(&person.name, &person.phone, move || {
                    model.navigate(Route::Contact(id))
                }));
            }
        }
        scroll(content.into())
    };
    let pad = ui.pivot;
    let people = vm.clone();
    let menu = ui.menu;
    let menu_vm = vm.clone();
    let bar = app_bar(
        vec![
            action(Icon::Keyboard, "keypad", move || {
                pad.set(if pad.get_untracked() == 2 { 0 } else { 2 })
            }),
            action(Icon::People, "phone book", move || {
                people.navigate(Route::App(AppId::People))
            }),
            action(Icon::More, "more", move || {
                menu.update(|open| *open = !*open)
            }),
        ],
        ui.menu.get(),
        vec![flat("settings", move || {
            menu_vm.navigate(Route::Setting("ringtones+sounds".into()))
        })],
    );
    page("phone", body, bar)
}

fn dialpad(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let keys = [
        ("1", ""),
        ("2", "abc"),
        ("3", "def"),
        ("4", "ghi"),
        ("5", "jkl"),
        ("6", "mno"),
        ("7", "pqrs"),
        ("8", "tuv"),
        ("9", "wxyz"),
        ("*", ""),
        ("0", "+"),
        ("#", ""),
    ];
    let mut pad = grid()
        .column_definitions("*,*,*")
        .row_definitions("72,72,72,72")
        .row_spacing(4.0)
        .column_spacing(5.0);
    for (index, (key, letters)) in keys.into_iter().enumerate() {
        let dial = ui.dial;
        pad=pad.child(rsx!{Button{Grid.row:(index/3)as i32,Grid.column:(index%3)as i32,class:"phone-flat",
            padding:0.0,horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Center,
            automation_properties_name:format!("{key} {letters}"),on_click:move |_|dial.update(|number|number.push_str(key)),
            StackPanel{horizontal_alignment:H::Center,spacing:0.0,
                TextBlock{text:key,font_size:38.0,horizontal_alignment:H::Center}
                TextBlock{text:letters,font_size:13.0,foreground:"#929292",horizontal_alignment:H::Center}
            }
        }});
    }
    let back = ui.dial;
    let call = vm.clone();
    let dial = ui.dial;
    let plus = ui.dial;
    let save = vm.clone();
    let save_ui = ui.clone();
    scroll(rsx! {StackPanel{margin:(18.0,12.0),spacing:12.0,
        Grid{column_definitions:"*,Auto",
            TextBox{class:"phone-input",text:ui.dial.two_way(),font_size:30.0,placeholder_text:"phone number",automation_properties_name:"Phone number"}
            Border{Grid.column:1,{flat("⌫",move ||back.update(|number|{number.pop();}))}}
        }
        {pad}
        Button{class:"phone-button",content:"call",font_size:25.0,background:vm.data().settings.accent.as_str(),foreground:"White",min_height:54.0,
            on_click:move |_|call.record_call(String::new(),dial.get_untracked())}
        StackPanel{orientation:Orientation::Horizontal,spacing:24.0,
            {flat("+",move ||plus.update(|number|number.push('+')))}
            {flat("save",move ||{
                let number=save_ui.dial.get_untracked();let model=save.clone();let error=save_ui.form_error;
                save_ui.form("new contact",vec![("name",String::new()),("phone",number)],Rc::new(move |values|{
                    let name=values.first().cloned().unwrap_or_default();
                    if name.trim().is_empty(){error.set("Enter a contact name.".into());return;}
                    model.save_contact(phone_domain::Contact{name,phone:values.get(1).cloned().unwrap_or_default(),..Default::default()});
                }));
            })}
        }
        {muted("Calls open your device's phone app.",14.0)}
    }})
}
