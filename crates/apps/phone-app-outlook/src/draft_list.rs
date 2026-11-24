use crate::{composer, drafts};
use ferroui_base::{layout::HorizontalAlignment as H, media::TextWrapping};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_sdk::{
    common::{empty, muted, page, scroll, title},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut body = stack_panel().child(title("drafts"));
    match drafts::read(&vm) {
        Err(error) => body = body.child(empty("Drafts unavailable", &error)),
        Ok(mut library) => {
            library.items.sort_by_key(|draft| {
                (
                    std::cmp::Reverse(draft.updated),
                    std::cmp::Reverse(draft.id),
                )
            });
            if library.items.is_empty() {
                body = body.child(empty(
                    "No drafts",
                    "Start a new email and choose save draft to keep it on this phone.",
                ));
            } else {
                body = body.child(rsx! {Border{margin:(20.0,0.0,20.0,10.0),{muted("Tap a draft to edit. Touch and hold to delete.",16.0)}}});
            }
            for draft in library.items {
                let open = ui.clone();
                let remove = ui.clone();
                let model = vm.clone();
                let id = draft.id;
                let subject = if draft.fields.subject.trim().is_empty() {
                    "(no subject)"
                } else {
                    &draft.fields.subject
                };
                let to = if draft.fields.to.trim().is_empty() {
                    "no recipient"
                } else {
                    &draft.fields.to
                };
                let preview = draft
                    .fields
                    .body
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
                    .chars()
                    .take(160)
                    .collect::<String>();
                let selected = draft.clone();
                body = body.child(rsx! {Button {
                    name:format!("MailDraft-{id}"),class:"phone-flat",padding:(20.0,12.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                    automation_properties_name:format!("Draft to {to}: {subject}"),
                    on_click:move |_| composer::begin(&open,selected.fields.clone(),Some(selected.clone())),
                    on_context_requested:move |event| {
                        event.set_handled(true);
                        let delete_vm=model.clone();
                        let delete_ui=remove.clone();
                        remove.form_with_action("delete this draft?",vec![],"delete",Rc::new(move |_| {
                            if let Err(error)=drafts::delete(&delete_vm,id){delete_ui.form_error.set(error);}
                        }));
                    },
                    StackPanel {spacing:4.0,
                        TextBlock {text:subject,font_size:28.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap}
                        {muted(&format!("to: {to}"),18.0)}
                        TextBlock {text:preview,font_size:17.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap,max_height:48.0}
                        {muted(&phone_domain::time_text(draft.updated.saturating_add(i64::from(vm.utc_offset())),vm.data().settings.time_24h),13.0)}
                    }
                }});
            }
        }
    }
    let add = ui.clone();
    let inbox = ui.clone();
    let folders = ui.clone();
    page(
        "Outlook",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Plus, "new", move || {
                composer::begin(&add, drafts::Fields::default(), None)
            })],
            ui.menu.get(),
            vec![
                flat("inbox", move || {
                    inbox.menu.set(false);
                    inbox.state::<crate::MailState>().folder.set("inbox".into());
                    inbox.state::<crate::MailState>().saved_drafts.set(false);
                }),
                flat("folders", move || {
                    folders.menu.set(false);
                    folders.state::<crate::MailState>().folders.set(true);
                }),
            ],
        ),
    )
}
