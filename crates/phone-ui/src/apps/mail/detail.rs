use super::compose;
use crate::{
    common::{muted, page, scroll, text},
    session::Session,
};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat};
use phone_domain::{Action, AppId, Mail, MailOperation, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;
pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, mail: Mail) -> Element {
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
    let permanent = mail.folder == "deleted";
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
    let mut commands = vec![
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
    ];
    if permanent {
        let restore = vm.clone();
        let state = ui.clone();
        commands.push(flat("move to inbox", move || {
            restore.dispatch(Action::MailBatch {
                ids: vec![id],
                operation: MailOperation::Restore,
            });
            state.mail_folder.set("inbox".into());
            state.pivot.set(0);
            state.query.set(String::new());
            state.menu.set(false);
            state.mail.reset();
            restore.navigate(Route::App(AppId::Outlook));
        }));
    }
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
                    if permanent {
                        "permanently delete this message?"
                    } else {
                        "delete this message?"
                    },
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
        commands,
    );
    page("Outlook", scroll(body), bar)
}
