use crate::{
    common::{empty, page, scroll, title},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, icon};
use phone_domain::{Action, Conversation, Route};
use phone_state::PhoneViewModel;
use std::{collections::BTreeSet, rc::Rc};

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let searching = ui.messages.searching.get();
    let selecting = ui.messages.selecting.get();
    let query = if searching {
        ui.query.get().to_lowercase()
    } else {
        String::new()
    };
    let mut conversations = vm
        .data()
        .conversations
        .iter()
        .filter(|conversation| {
            query.is_empty()
                || vm
                    .data()
                    .contact_name(conversation)
                    .to_lowercase()
                    .contains(&query)
                || conversation.recipient.to_lowercase().contains(&query)
                || conversation
                    .messages
                    .iter()
                    .any(|message| message.text.to_lowercase().contains(&query))
        })
        .cloned()
        .collect::<Vec<_>>();
    conversations.sort_by_key(|conversation| std::cmp::Reverse(conversation.timestamp()));
    let ids = conversations
        .iter()
        .map(|conversation| conversation.id)
        .collect::<BTreeSet<_>>();
    let rows = conversations
        .into_iter()
        .map(|conversation| thread_row(vm.clone(), ui.clone(), conversation))
        .collect::<Vec<_>>();
    let count = ui.messages.selected.get().len();
    let heading = if selecting {
        format!("{count} selected")
    } else {
        "threads".into()
    };
    let bar = if selecting {
        selection_bar(vm, ui.clone(), ids)
    } else {
        normal_bar(vm, ui.clone())
    };
    page(
        "messaging",
        scroll(rsx! {StackPanel{
            {title(&heading)}
            if searching{TextBox{name:"MessageSearch",class:"phone-input",text:ui.query.two_way(),placeholder_text:"search messages",automation_properties_name:"Search messages",margin:(20.0,0.0,20.0,12.0)}}
            if rows.is_empty(){{empty("No messages",if searching{"Try a name, number or word from a message."}else{"Tap new to start a conversation."})}}
            for row in rows{{row}}
        }}),
        bar,
    )
}

fn thread_row(vm: Rc<PhoneViewModel>, ui: Rc<Session>, conversation: Conversation) -> Element {
    let id = conversation.id;
    let name = vm.data().contact_name(&conversation);
    let settings = vm.data().settings.clone();
    let color = if conversation.unread && !settings.high_contrast {
        settings.accent.clone()
    } else {
        settings.foreground().into()
    };
    let preview = conversation.preview().chars().take(180).collect::<String>();
    let drafting = ui
        .message_drafts
        .get()
        .get(&id)
        .is_some_and(|value| !value.is_empty());
    let preview = if drafting {
        format!(
            "draft: {}",
            ui.message_drafts
                .get()
                .get(&id)
                .cloned()
                .unwrap_or_default()
        )
    } else {
        preview
    };
    let selecting = ui.messages.selecting.get();
    let selected = ui.messages.selected.get().contains(&id);
    let check = ui.clone();
    let open = ui.clone();
    let context = ui.clone();
    let model = vm.clone();
    rsx! {Grid{key:format!("thread-{id}"),column_definitions:if selecting{"52,*"}else{"20,*"},
        Button{class:"phone-flat",padding:0.0,horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,
            automation_properties_name:format!("{} thread with {name}",if selected{"Deselect"}else{"Select"}),
            on_click:move |_|{if check.messages.selecting.get_untracked(){check.messages.toggle(id);}else{check.messages.begin(Some(id));}},
            if selecting{Border{width:25.0,height:25.0,border_thickness:2.0,border_brush:settings.foreground(),if selected{{icon(Icon::Check,18.0,settings.foreground())}}}}
        }
        Button{Grid.column:1,name:format!("Context-Thread-{id}"),class:"phone-flat",padding:(0.0,14.0,20.0,14.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("{name}: {preview}"),
            on_context_requested:move |event|{event.set_handled(true);context.messages.begin(Some(id));},
            on_click:move |_|{if open.messages.selecting.get_untracked(){open.messages.toggle(id);}else{model.navigate(Route::Conversation(id));}},
            StackPanel{spacing:4.0,
                TextBlock{text:name,font_size:29.0*phone_controls::text_scale(),foreground:color.as_str(),font_weight:if conversation.unread{FontWeight::SemiBold}else{FontWeight::Light}}
                TextBlock{text:preview,font_size:19.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap,max_height:50.0,foreground:if drafting&&!settings.high_contrast{settings.accent.as_str()}else{settings.secondary()}}
                TextBlock{text:phone_domain::time_text(conversation.timestamp().saturating_add(i64::from(vm.utc_offset())),settings.time_24h),font_size:13.0,foreground:settings.secondary()}
            }
        }
    }}
}

fn normal_bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let add = ui.clone();
    let select = ui.clone();
    let search = ui.clone();
    let menu = ui.menu;
    let settings = vm.clone();
    let compose_vm = vm.clone();
    let compose_ui = ui.clone();
    app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                add.query.set(String::new());
                add.messages.contacts.set(true);
            }),
            action(Icon::Check, "select", move || select.messages.begin(None)),
            action(Icon::Search, "search", move || {
                search.query.set(String::new());
                search.messages.searching.update(|value| *value = !*value);
            }),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![
            flat("new phone number", move || {
                super::compose(compose_vm.clone(), compose_ui.clone(), String::new())
            }),
            flat("settings", move || {
                settings.navigate(Route::Setting("messaging".into()))
            }),
        ],
    )
}

fn selection_bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>, ids: BTreeSet<u64>) -> Element {
    let remove_vm = vm.clone();
    let remove_ui = ui.clone();
    let read_vm = vm.clone();
    let read_ui = ui.clone();
    let close = ui.clone();
    let menu = ui.menu;
    let all = ui.clone();
    app_bar(
        vec![
            action(Icon::Delete, "delete", move || {
                let selected = remove_ui.messages.selected.get_untracked();
                if selected.is_empty() {
                    return;
                }
                let model = remove_vm.clone();
                let state = remove_ui.clone();
                remove_ui.confirm(
                    &format!(
                        "delete {} thread{}?",
                        selected.len(),
                        if selected.len() == 1 { "" } else { "s" }
                    ),
                    Rc::new(move || apply(&model, &state, true)),
                );
            }),
            action(Icon::Message, "mark read", move || {
                apply(&read_vm, &read_ui, false)
            }),
            action(Icon::Close, "cancel", move || close.messages.reset()),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![flat("select all", move || {
            all.messages.selected.set(ids.clone())
        })],
    )
}

fn apply(vm: &PhoneViewModel, ui: &Session, delete: bool) {
    let ids = ui
        .messages
        .selected
        .get_untracked()
        .into_iter()
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return;
    }
    if delete {
        ui.message_drafts.update(|drafts| {
            for id in &ids {
                drafts.remove(id);
            }
        });
    }
    vm.dispatch(Action::ConversationBatch { ids, delete });
    ui.messages.reset();
    ui.menu.set(false);
}
