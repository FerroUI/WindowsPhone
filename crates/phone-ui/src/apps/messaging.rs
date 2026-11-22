use crate::{
    common::{empty, page, scroll, title},
    session::Session,
};
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat};
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
        let preview=conversation.preview().chars().take(180).collect::<String>();
        rsx!{Button{class:"phone-flat",padding:(20.0,14.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("{name}: {preview}"),on_click:move |_|model.navigate(Route::Conversation(id)),
            StackPanel{spacing:4.0,
                TextBlock{text:name,font_size:29.0*phone_controls::text_scale(),foreground:color.as_str(),font_weight:if conversation.unread{FontWeight::SemiBold}else{FontWeight::Light}}
                TextBlock{text:preview,font_size:19.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap,max_height:50.0,foreground:"#929292"}
                TextBlock{text:phone_domain::time_text(conversation.timestamp().saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h),font_size:13.0,foreground:"#929292"}
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
        let time=phone_domain::time_text(message.timestamp.saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h);
        let caption=if message.draft{format!("{time} · draft opened in messaging app")}else{time};
        let fill=if message.incoming{if vm.data().settings.dark{"#333333".into()}else{"#DDDDDD".into()}}else{vm.data().settings.accent.clone()};
        rsx!{Border{background:fill.as_str(),margin:if message.incoming{(0.0,6.0,32.0,6.0)}else{(32.0,6.0,0.0,6.0)},padding:14.0,
            StackPanel{spacing:8.0,
                TextBlock{text:message.text.clone(),font_size:21.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap,foreground:if message.incoming{vm.data().settings.foreground()}else{"White"}}
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
    let drafts = ui.message_drafts;
    let draft = drafts.get().get(&id).cloned().unwrap_or_default();
    let body = rsx! {Grid{row_definitions:"Auto,*,Auto",
        TextBlock{text:name,font_size:34.0,margin:(20.0,0.0,20.0,14.0),text_wrapping:TextWrapping::Wrap}
        ScrollViewer{Grid.row:1,key:format!("conversation-{id}-{}",conversation.messages.len()),name:format!("ConversationMessages-{id}"),
            horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
            on_loaded:move |event|if let Some(scroll)=event.source().and_then(|source|source.cast::<ScrollViewer>()){scroll.scroll_to_end();},
            StackPanel{margin:(20.0,0.0),for bubble in bubbles{{bubble}}}
        }
        Grid{Grid.row:2,column_definitions:"*,Auto",margin:(14.0,8.0),column_spacing:6.0,
            TextBox{key:format!("composer-{id}"),name:format!("ConversationInput-{id}"),class:"phone-input",text:draft,placeholder_text:"type a message",automation_properties_name:"Message",accepts_return:true,text_wrapping:TextWrapping::Wrap,min_height:50.0,max_height:130.0,
                on_text_changed:move |event|if let Some(input)=event.source().and_then(|source|source.cast::<TextBox>()){
                    let text=input.text().unwrap_or_default();
                    if drafts.get_untracked().get(&id).map(String::as_str).unwrap_or("")!=text {
                        drafts.update(|values|{if text.is_empty(){values.remove(&id);}else{values.insert(id,text);}});
                    }
                }
            }
            Border{Grid.column:1,{action(Icon::Send,"send",move ||{
                let text=send_ui.message_drafts.get_untracked().get(&id).cloned().unwrap_or_default();
                if !text.trim().is_empty(){send_ui.message_drafts.update(|values|{values.remove(&id);});send_vm.send_message(id,text);}
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
            let drafts = confirm.message_drafts;
            confirm.confirm(
                "delete this thread?",
                Rc::new(move || {
                    drafts.update(|values| {
                        values.remove(&id);
                    });
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
                error.set(vm.status());
                return;
            }
            vm.navigate(Route::Conversation(id));
            vm.send_message(id, body);
        }),
    );
}
