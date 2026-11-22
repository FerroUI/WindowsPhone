use super::compose;
use crate::{
    common::{empty, muted, page, row, scroll, title},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, icon, pivot};
use phone_domain::{Action, Mail, MailOperation, Route};
use phone_state::PhoneViewModel;
use std::{collections::BTreeSet, rc::Rc};

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui.mail.folders.get() {
        return folders(vm, ui);
    }
    let mode = ui.pivot.get().min(2);
    let folder = ui.mail_folder.get();
    let selecting = ui.mail.selecting.get();
    let searching = ui.mail.searching.get();
    let query = if searching {
        ui.query.get().to_lowercase()
    } else {
        String::new()
    };
    let mut messages = vm
        .data()
        .mail
        .iter()
        .filter(|mail| {
            mail.folder == folder && (mode != 1 || !mail.read) && (mode != 2 || mail.flagged)
        })
        .filter(|mail| {
            query.is_empty()
                || mail.from.to_lowercase().contains(&query)
                || mail.subject.to_lowercase().contains(&query)
                || mail.body.to_lowercase().contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    messages.sort_by_key(|mail| std::cmp::Reverse(mail.timestamp));
    let ids = messages.iter().map(|mail| mail.id).collect::<BTreeSet<_>>();
    let rows = messages
        .into_iter()
        .map(|mail| mail_row(vm.clone(), ui.clone(), mail))
        .collect::<Vec<_>>();
    let pivots = ["all", "unread", "flagged"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let state = ui.clone();
            pivot(label, mode == index, move || {
                state.pivot.set(index);
                state.mail.selected.set(BTreeSet::new());
            })
        })
        .collect::<Vec<_>>();
    let count = ui.mail.selected.get().len();
    let heading = if selecting {
        format!("{count} selected")
    } else {
        folder_label(&folder).to_owned()
    };
    let bar = if selecting {
        selection_bar(vm.clone(), ui.clone(), ids, &folder)
    } else {
        normal_bar(vm.clone(), ui.clone())
    };
    page(
        "Outlook",
        scroll(rsx! {StackPanel{
            {title(&heading)}
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0),for item in pivots{{item}}}
            if searching{TextBox{name:"MailSearch",class:"phone-input",text:ui.query.two_way(),placeholder_text:"search mail",automation_properties_name:"Search mail",margin:(20.0,6.0,20.0,10.0)}}
            if rows.is_empty(){{empty("No messages",if searching{"Try a different sender, subject or word."}else{"There are no messages in this view."})}}
            for item in rows{{item}}
        }}),
        bar,
    )
}

fn mail_row(vm: Rc<PhoneViewModel>, ui: Rc<Session>, mail: Mail) -> Element {
    let id = mail.id;
    let selecting = ui.mail.selecting.get();
    let selected = ui.mail.selected.get().contains(&id);
    let settings = vm.data().settings.clone();
    let foreground = if mail.read || settings.high_contrast {
        settings.foreground().to_owned()
    } else {
        settings.accent.clone()
    };
    let preview = mail
        .body
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(180)
        .collect::<String>();
    let select = ui.clone();
    let open = ui.clone();
    let context = ui.clone();
    let model = vm.clone();
    let from = mail.from.clone();
    rsx! {Grid{key:format!("mail-{id}"),column_definitions:if selecting{"52,*"}else{"20,*"},
        Button {class:"phone-flat",padding:0.0,horizontal_alignment:H::Stretch,vertical_alignment:V::Stretch,
            automation_properties_name:format!("{} email from {from}",if selected{"Deselect"}else{"Select"}),
            on_click:move |_|{if select.mail.selecting.get_untracked(){select.mail.toggle(id);}else{select.mail.begin(Some(id));}},
            if selecting{Border{width:25.0,height:25.0,border_thickness:2.0,border_brush:settings.foreground(),
                if selected{{icon(Icon::Check,18.0,settings.foreground())}}}}
        }
        Button {Grid.column:1,name:format!("Context-Mail-{id}"),class:"phone-flat",padding:(0.0,12.0,20.0,12.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("{}: {}",mail.from,mail.subject),
            on_context_requested:move |event|{event.set_handled(true);context.mail.begin(Some(id));},
            on_click:move |_|{if open.mail.selecting.get_untracked(){open.mail.toggle(id);}else{model.navigate(Route::Mail(id));}},
            StackPanel{spacing:3.0,
                TextBlock{text:format!("{}{}",mail.from,if mail.flagged{"  ⚑"}else{""}),font_size:28.0*phone_controls::text_scale(),foreground:foreground.as_str(),font_weight:if mail.read{FontWeight::Light}else{FontWeight::SemiBold}}
                TextBlock{text:mail.subject,font_size:21.0*phone_controls::text_scale(),foreground:foreground.as_str(),text_wrapping:TextWrapping::Wrap}
                TextBlock{text:preview,font_size:17.0*phone_controls::text_scale(),foreground:settings.secondary(),text_wrapping:TextWrapping::Wrap,max_height:48.0}
                {muted(&phone_domain::time_text(mail.timestamp.saturating_add(i64::from(vm.utc_offset())),settings.time_24h),13.0)}
            }
        }
    }}
}

fn normal_bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let select = ui.clone();
    let search = ui.clone();
    let menu = ui.menu;
    let folders = ui.clone();
    let notifications = vm.clone();
    app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                compose(
                    add_vm.clone(),
                    add_ui.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            }),
            action(Icon::Check, "select", move || select.mail.begin(None)),
            action(Icon::Search, "search", move || {
                search.query.set(String::new());
                search.mail.searching.update(|value| *value = !*value);
            }),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![
            flat("folders", move || {
                folders.menu.set(false);
                folders.mail.folders.set(true);
            }),
            flat("notification settings", move || {
                notifications.navigate(Route::Setting("notification:Outlook".into()))
            }),
        ],
    )
}

fn selection_bar(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    ids: BTreeSet<u64>,
    folder: &str,
) -> Element {
    let remove_vm = vm.clone();
    let remove_ui = ui.clone();
    let read_vm = vm.clone();
    let read_ui = ui.clone();
    let close = ui.clone();
    let menu = ui.menu;
    let all = ui.clone();
    let unread_vm = vm.clone();
    let unread_ui = ui.clone();
    let flag_vm = vm.clone();
    let flag_ui = ui.clone();
    let unflag_vm = vm.clone();
    let unflag_ui = ui.clone();
    let restore_vm = vm.clone();
    let restore_ui = ui.clone();
    let permanent = folder == "deleted";
    let mut commands = vec![
        flat("select all", move || all.mail.selected.set(ids.clone())),
        flat("mark as unread", move || {
            apply(&unread_vm, &unread_ui, MailOperation::Unread)
        }),
        flat("flag", move || {
            apply(&flag_vm, &flag_ui, MailOperation::Flag)
        }),
        flat("remove flag", move || {
            apply(&unflag_vm, &unflag_ui, MailOperation::Unflag)
        }),
    ];
    if permanent {
        commands.push(flat("move to inbox", move || {
            apply(&restore_vm, &restore_ui, MailOperation::Restore)
        }));
    }
    app_bar(
        vec![
            action(Icon::Delete, "delete", move || {
                let selected = remove_ui.mail.selected.get_untracked();
                if selected.is_empty() {
                    return;
                }
                let model = remove_vm.clone();
                let state = remove_ui.clone();
                remove_ui.confirm(
                    &format!(
                        "{} {} message{}?",
                        if permanent {
                            "permanently delete"
                        } else {
                            "delete"
                        },
                        selected.len(),
                        if selected.len() == 1 { "" } else { "s" }
                    ),
                    Rc::new(move || apply(&model, &state, MailOperation::Delete)),
                );
            }),
            action(Icon::Mail, "mark read", move || {
                apply(&read_vm, &read_ui, MailOperation::Read)
            }),
            action(Icon::Close, "cancel", move || close.mail.reset()),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        commands,
    )
}

fn apply(vm: &PhoneViewModel, ui: &Session, operation: MailOperation) {
    let ids = ui
        .mail
        .selected
        .get_untracked()
        .into_iter()
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return;
    }
    vm.dispatch(Action::MailBatch { ids, operation });
    ui.mail.reset();
    ui.menu.set(false);
}

fn folders(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut names = vm
        .data()
        .mail
        .iter()
        .map(|mail| mail.folder.clone())
        .collect::<BTreeSet<_>>();
    names.insert("inbox".into());
    names.insert("deleted".into());
    let mut body = stack_panel().child(title("folders"));
    for name in names {
        let count = vm
            .data()
            .mail
            .iter()
            .filter(|mail| mail.folder == name)
            .count();
        let unread = vm
            .data()
            .mail
            .iter()
            .filter(|mail| mail.folder == name && !mail.read)
            .count();
        let state = ui.clone();
        let label = folder_label(&name).to_owned();
        body = body.child(row(
            &label,
            &format!("{count} messages, {unread} unread"),
            move || {
                state.mail_folder.set(name.clone());
                state.mail.reset();
                state.pivot.set(0);
                state.query.set(String::new());
            },
        ));
    }
    let close = ui.clone();
    page(
        "Outlook",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Back, "back", move || {
                close.mail.folders.set(false)
            })],
            false,
            vec![],
        ),
    )
}

fn folder_label(name: &str) -> &str {
    if name == "deleted" {
        "deleted items"
    } else {
        name
    }
}
