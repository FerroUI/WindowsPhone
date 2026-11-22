use crate::{
    common::{empty, muted, page, scroll, text, title},
    session::Session,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat, pivot};
use phone_domain::{AppId, Mail, Route};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Mail(id) = vm.route() {
        let mail = vm.data().mail(id).cloned();
        if let Some(mail) = mail {
            return detail(vm, ui, mail);
        }
    }
    let mode = ui.pivot.get().min(2);
    let folder = if ui.mail_folder.get() == "deleted" {
        "deleted"
    } else {
        "inbox"
    };
    let query = ui.query.get().to_lowercase();
    let mut messages = vm
        .data()
        .mail
        .iter()
        .filter(|mail| {
            mail.folder == folder && (mode != 1 || !mail.read) && (mode != 2 || mail.flagged)
        })
        .filter(|mail| {
            format!("{} {} {}", mail.from, mail.subject, mail.body)
                .to_lowercase()
                .contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    messages.sort_by_key(|mail| std::cmp::Reverse(mail.timestamp));
    let rows=messages.into_iter().map(|mail|{
        let model=vm.clone();let id=mail.id;let foreground=if mail.read{vm.data().settings.foreground().to_owned()}else{vm.data().settings.accent.clone()};
        let preview=mail.body.lines().find(|line|!line.trim().is_empty()).unwrap_or("").chars().take(180).collect::<String>();
        rsx!{Button{class:"phone-flat",padding:(20.0,12.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("{}: {}",mail.from,mail.subject),
            on_click:move |_|model.navigate(Route::Mail(id)),
            StackPanel{spacing:3.0,
                TextBlock{text:format!("{}{}",mail.from,if mail.flagged{"  ⚑"}else{""}),font_size:28.0*phone_controls::text_scale(),foreground:foreground.as_str(),font_weight:if mail.read{FontWeight::Light}else{FontWeight::SemiBold}}
                TextBlock{text:mail.subject,font_size:21.0*phone_controls::text_scale(),foreground:foreground.as_str(),text_wrapping:TextWrapping::Wrap}
                TextBlock{text:preview,font_size:17.0*phone_controls::text_scale(),foreground:"#929292",text_wrapping:TextWrapping::Wrap,max_height:48.0}
                {muted(&phone_domain::time_text(mail.timestamp.saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h),13.0)}
            }
        }}
    }).collect::<Vec<_>>();
    let pivots = ["all", "unread", "flagged"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let state = ui.pivot;
            pivot(label, mode == index, move || state.set(index))
        })
        .collect::<Vec<_>>();
    let compose_vm = vm.clone();
    let compose_ui = ui.clone();
    let menu = ui.menu;
    let inbox = ui.clone();
    let deleted = ui.clone();
    let bar = app_bar(
        vec![
            action(Icon::Plus, "new", move || {
                compose(
                    compose_vm.clone(),
                    compose_ui.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            }),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![
            flat("inbox", move || {
                inbox.mail_folder.set("inbox".into());
                inbox.menu.set(false);
            }),
            flat("deleted items", move || {
                deleted.mail_folder.set("deleted".into());
                deleted.menu.set(false);
            }),
        ],
    );
    page(
        "Outlook",
        scroll(rsx! {StackPanel{
            {title(if folder=="inbox"{"inbox"}else{"deleted"})}
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0),for item in pivots{{item}}}
            TextBox{class:"phone-input",text:ui.query.two_way(),placeholder_text:"search mail",automation_properties_name:"Search mail",margin:(20.0,6.0,20.0,10.0)}
            if rows.is_empty(){{empty("No messages","There are no messages in this view.")}}
            for item in rows{{item}}
        }}),
        bar,
    )
}

fn detail(vm: Rc<PhoneViewModel>, ui: Rc<Session>, mail: Mail) -> Element {
    let read = vm.clone();
    let id = mail.id;
    let reply_vm = vm.clone();
    let reply_ui = ui.clone();
    let reply_address = mail.address.clone();
    let reply_subject = if mail.subject.to_lowercase().starts_with("re:") {
        mail.subject.clone()
    } else {
        format!("Re: {}", mail.subject)
    };
    let flag = vm.clone();
    let delete = vm.clone();
    let confirm = ui.clone();
    let menu = ui.menu;
    let unread = vm.clone();
    let forwarded_vm = vm.clone();
    let forwarded_ui = ui.clone();
    let forwarded_subject = format!("Fwd: {}", mail.subject);
    let forwarded_body = format!(
        "\n\nFrom: {}\nSubject: {}\n\n{}",
        mail.from, mail.subject, mail.body
    );
    let body = rsx! {StackPanel{margin:20.0,spacing:16.0,on_loaded:move |_|read.mark_mail(id,true),
        {text(&mail.subject,35.0)}
        {text(&mail.from,25.0)}
        {muted(&mail.address,16.0)}
        {muted(&phone_domain::time_text(mail.timestamp.saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h),14.0)}
        TextBlock{text:mail.body,font_size:21.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap,line_height:30.0*phone_controls::text_scale()}
    }};
    let bar = app_bar(
        vec![
            action(Icon::Undo, "reply", move || {
                compose(
                    reply_vm.clone(),
                    reply_ui.clone(),
                    reply_address.clone(),
                    reply_subject.clone(),
                    String::new(),
                )
            }),
            action(
                Icon::Star,
                if mail.flagged { "unflag" } else { "flag" },
                move || flag.toggle_flag_mail(id),
            ),
            action(Icon::Delete, "delete", move || {
                let model = delete.clone();
                confirm.confirm(
                    "delete this message?",
                    Rc::new(move || {
                        model.delete_mail(id);
                        model.navigate(Route::App(AppId::Outlook));
                    }),
                );
            }),
            action(Icon::More, "more", move || {
                menu.update(|value| *value = !*value)
            }),
        ],
        ui.menu.get(),
        vec![
            flat("mark as unread", move || {
                unread.mark_mail(id, false);
                unread.navigate(Route::App(AppId::Outlook));
            }),
            flat("forward", move || {
                compose(
                    forwarded_vm.clone(),
                    forwarded_ui.clone(),
                    String::new(),
                    forwarded_subject.clone(),
                    forwarded_body.clone(),
                )
            }),
        ],
    );
    page("Outlook", scroll(body), bar)
}

fn compose(vm: Rc<PhoneViewModel>, ui: Rc<Session>, to: String, subject: String, body: String) {
    let error = ui.form_error;
    ui.form(
        "new email",
        vec![("to", to), ("subject", subject), ("body", body)],
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
