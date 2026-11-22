use crate::{
    common::{empty, muted, page, scroll, text, title},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_domain::{AppId, Conversation, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Conversation(id) = vm.route() {
        let conversation = vm.data().conversation(id).cloned();
        if let Some(conversation) = conversation {
            return thread(vm, ui, conversation);
        }
    }
    let mut conversations = vm.data().conversations.clone();
    conversations.sort_by_key(|conversation| std::cmp::Reverse(conversation.timestamp()));
    let rows=conversations.iter().map(|conversation|{
        let name=vm.data().contact_name(conversation);let model=vm.clone();let id=conversation.id;
        let color=if conversation.unread{vm.data().settings.accent.clone()}else{vm.data().settings.foreground().into()};
        let preview=conversation.preview().to_owned();
        rsx!{Button{class:"phone-flat",padding:(20.0,14.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("{name}: {preview}"),on_click:move |_|{model.mark_conversation_read(id);model.navigate(Route::Conversation(id));},
            StackPanel{spacing:4.0,
                TextBlock{text:name,font_size:29.0,foreground:color.as_str(),font_weight:if conversation.unread{FontWeight::SemiBold}else{FontWeight::Light}}
                TextBlock{text:preview,font_size:19.0,text_wrapping:TextWrapping::Wrap,max_height:50.0,foreground:"#929292"}
                TextBlock{text:phone_domain::time_text(conversation.timestamp(),vm.data().settings.time_24h),font_size:13.0,foreground:"#929292"}
            }
        }}
    }).collect::<Vec<_>>();
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let menu = ui.menu;
    let settings = vm.clone();
    let bar = app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                compose(add_vm.clone(), add_ui.clone(), String::new())
            }),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![flat("settings", move || {
            settings.navigate(Route::Setting("messaging".into()))
        })],
    );
    page(
        "messaging",
        scroll(
            rsx! {StackPanel{{title("threads")}if rows.is_empty(){{empty("No messages","Tap new to start a conversation.")}}for item in rows{{item}}}},
        ),
        bar,
    )
}

fn thread(vm: Rc<PhoneViewModel>, ui: Rc<Session>, conversation: Conversation) -> Element {
    let name = vm.data().contact_name(&conversation);
    let id = conversation.id;
    let bubbles=conversation.messages.iter().map(|message|{
        let time=phone_domain::time_text(message.timestamp,vm.data().settings.time_24h);
        let caption=if message.draft{format!("{time} · draft opened in messaging app")}else{time};
        let fill=if message.incoming{if vm.data().settings.dark{"#333333".into()}else{"#DDDDDD".into()}}else{vm.data().settings.accent.clone()};
        rsx!{Border{background:fill.as_str(),margin:if message.incoming{(0.0,6.0,32.0,6.0)}else{(32.0,6.0,0.0,6.0)},padding:14.0,
            StackPanel{spacing:8.0,
                TextBlock{text:message.text.clone(),font_size:21.0,text_wrapping:TextWrapping::Wrap,foreground:if message.incoming{vm.data().settings.foreground()}else{"White"}}
                TextBlock{text:caption,font_size:12.0,text_wrapping:TextWrapping::Wrap,foreground:if message.incoming{"#A6A6A6"}else{"#EEEEEE"}}
            }
        }}
    }).collect::<Vec<_>>();
    let send_vm = vm.clone();
    let send_ui = ui.clone();
    let call = vm.clone();
    let number = conversation.recipient.clone();
    let call_name = name.clone();
    let remove = vm.clone();
    let confirm = ui.clone();
    let menu = ui.menu;
    let read = vm.clone();
    let entered = ui.clone();
    let body = rsx! {Grid{row_definitions:"Auto,*,Auto",on_loaded:move |_|{
        read.mark_conversation_read(id);
        if entered.selected.get_untracked()!=Some(id){entered.selected.set(Some(id));entered.entry.set(String::new());}
    },
        TextBlock{text:name,font_size:34.0,margin:(20.0,0.0,20.0,14.0),text_wrapping:TextWrapping::Wrap}
        Border{Grid.row:1,{scroll(rsx!{StackPanel{margin:(20.0,0.0),for bubble in bubbles{{bubble}}}})}}
        Grid{Grid.row:2,column_definitions:"*,Auto",margin:(14.0,8.0),column_spacing:6.0,
            TextBox{class:"phone-input",text:ui.entry.two_way(),placeholder_text:"type a message",automation_properties_name:"Message",accepts_return:true,text_wrapping:TextWrapping::Wrap,min_height:50.0,max_height:130.0}
            Border{Grid.column:1,{action(Icon::Send,"send",move ||{
                let text=send_ui.entry.get_untracked();if !text.trim().is_empty(){send_ui.entry.set(String::new());send_vm.send_message(id,text);}
            })}}
        }
    }};
    let bar = app_bar(
        vec![
            action(Icon::Phone, "call", move || {
                call.record_call(call_name.clone(), number.clone())
            }),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![flat("delete thread", move || {
            let model = remove.clone();
            confirm.confirm(
                "delete this thread?",
                Rc::new(move || {
                    model.delete_conversation(id);
                    model.navigate(Route::App(AppId::Messaging));
                }),
            );
        })],
    );
    page("messaging", body, bar)
}

pub fn compose(vm: Rc<PhoneViewModel>, ui: Rc<Session>, recipient: String) {
    let error = ui.form_error;
    ui.form(
        "new message",
        vec![("to", recipient), ("message", String::new())],
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
                return;
            }
            vm.navigate(Route::Conversation(id));
            vm.send_message(id, body);
        }),
    );
}
