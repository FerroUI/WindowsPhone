//! Carrier messages use their native store/identity and stay out of phone backups.
//! Only an explicit Send press submits a message; activation only prepares a draft.
use ferroui_base::{
    layout::{HorizontalAlignment as H, Orientation},
    media::TextWrapping,
};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_sdk::{
    common::{empty, muted, page, scroll, text},
    device,
    host::platform::*,
    session::Session,
};
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct State {
    pub thread: Signal<Option<String>>,
    pub composing: Signal<bool>,
    pub to: Signal<String>,
    pub body: Signal<String>,
    pub subject: Signal<String>,
    pub multimedia: Signal<bool>,
    pub attachments: Signal<Vec<DeviceAttachment>>,
    pub query: Signal<String>,
    pub subscription: Signal<Option<String>>,
    pub pending: Signal<Option<String>>,
    picker: Signal<Option<String>>,
    download: RefCell<Option<(String, String, u64)>>,
    readable: Cell<bool>,
    pub context: Signal<Option<String>>,
    pub local: Signal<bool>,
    adopted: RefCell<String>,
    serial: Cell<u64>,
    attachment_revision: Cell<u64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            thread: signal(None),
            composing: signal(false),
            to: signal(String::new()),
            body: signal(String::new()),
            subject: signal(String::new()),
            multimedia: signal(false),
            attachments: signal(Vec::new()),
            query: signal(String::new()),
            subscription: signal(None),
            pending: signal(None),
            picker: signal(None),
            download: RefCell::new(None),
            readable: Cell::new(false),
            context: signal(None),
            local: signal(false),
            adopted: RefCell::new(String::new()),
            serial: Cell::new(0),
            attachment_revision: Cell::new(0),
        }
    }
}
pub fn reset(ui: &Rc<Session>) {
    let s = ui.state::<State>();
    s.thread.set(None);
    s.composing.set(false);
    s.to.set(String::new());
    s.body.set(String::new());
    s.subject.set(String::new());
    s.multimedia.set(false);
    s.attachments.set(Vec::new());
    s.attachment_revision.set(0);
    s.query.set(String::new());
    s.pending.set(None);
    s.picker.set(None);
    s.download.borrow_mut().take();
    s.readable.set(false);
    s.context.set(None);
    s.local.set(false);
    s.adopted.borrow_mut().clear();
}
pub fn back(ui: &Rc<Session>) -> bool {
    let s = ui.state::<State>();
    if s.context.get_untracked().is_some() {
        s.context.set(None);
    } else if s.composing.get_untracked() {
        s.composing.set(false);
    } else if s.thread.get_untracked().is_some() {
        s.thread.set(None);
        s.download.borrow_mut().take();
    } else {
        return false;
    }
    true
}
pub fn compose(vm: &PhoneViewModel, ui: &Rc<Session>, recipients: Vec<String>, body: String) {
    let s = ui.state::<State>();
    discard_attachments(vm, &s);
    s.pending.set(None);
    s.download.borrow_mut().take();
    s.subject.set(String::new());
    s.multimedia.set(false);
    s.local.set(false);
    s.thread.set(None);
    s.to.set(recipients.join(", "));
    s.body.set(body);
    s.composing.set(true);
    vm.navigate(phone_domain::Route::App(phone_domain::AppId::Messaging));
}
pub fn refresh(vm: &PhoneViewModel, ui: &Rc<Session>) {
    if !vm.platform_state().supports(HostFeature::Messaging) {
        return;
    }
    vm.refresh_device();
    let snapshot = vm.device_state().snapshot;
    if snapshot.supports(DeviceOperation::ReadMessages) {
        vm.request_device(DeviceRequest::MessageThreads { limit: 200 });
        if let Some(thread_id) = ui.state::<State>().thread.get_untracked() {
            vm.request_device(DeviceRequest::Messages {
                thread_id,
                limit: 200,
            });
        }
    }
}
pub fn tick(vm: &PhoneViewModel, ui: &Rc<Session>) {
    let s = ui.state::<State>();
    let state = vm.device_state();
    let download = s.download.borrow().clone();
    if let Some((message, original_thread, started_at)) = download {
        if let Some((_, thread_id)) = state
            .download_results
            .get(&message)
            .filter(|(revision, _)| *revision > started_at)
        {
            s.download.borrow_mut().take();
            if s.thread.get_untracked().as_deref() == Some(original_thread.as_str())
                && !s.composing.get_untracked()
                && !s.local.get_untracked()
            {
                s.thread.set(Some(thread_id.clone()));
                s.context.set(None);
                vm.request_device(DeviceRequest::Messages {
                    thread_id: thread_id.clone(),
                    limit: 200,
                });
            }
        }
    }
    let readable = state.snapshot.supports(DeviceOperation::ReadMessages);
    if readable != s.readable.replace(readable) && readable {
        refresh(vm, ui);
    }
    if state.draft_attachment_revision != s.attachment_revision.get() {
        s.attachment_revision.set(state.draft_attachment_revision);
        let owns = s.picker.get_untracked().is_some()
            && s.picker.get_untracked() == state.draft_attachment_client;
        if owns {
            s.picker.set(None);
        }
        if !owns {
            let ids = state
                .draft_attachments
                .iter()
                .map(|v| v.id.clone())
                .collect::<Vec<_>>();
            if !ids.is_empty() {
                vm.request_device(DeviceRequest::DiscardMessageAttachments { ids });
            }
        } else if !state.draft_attachment_cancelled {
            let old = s
                .attachments
                .get_untracked()
                .into_iter()
                .map(|v| v.id)
                .filter(|id| !state.draft_attachments.iter().any(|v| &v.id == id))
                .collect::<Vec<_>>();
            if !old.is_empty() {
                vm.request_device(DeviceRequest::DiscardMessageAttachments { ids: old });
            }
            s.attachments.set(state.draft_attachments.clone());
            if !state.draft_attachments.is_empty() {
                s.multimedia.set(true);
            }
        }
    }
    if let Some(draft) = state.compose_draft {
        if *s.adopted.borrow() != draft.id {
            discard_attachments(vm, &s);
            s.pending.set(None);
            s.subject.set(String::new());
            s.multimedia.set(false);
            *s.adopted.borrow_mut() = draft.id;
            s.to.set(draft.recipients.join(", "));
            s.body.set(draft.body);
            s.composing.set(true);
            s.local.set(false);
        }
    }
    if let Some(id) = s.pending.get_untracked() {
        if let Some((_, status)) = state.send_results.get(&id) {
            s.pending.set(None);
            if matches!(
                status,
                MessageDelivery::Queued
                    | MessageDelivery::Sending
                    | MessageDelivery::Sent
                    | MessageDelivery::Delivered
            ) {
                s.body.set(String::new());
                s.subject.set(String::new());
                // Transport owns its accepted attachment copies. Release only
                // the draft lease; native providers retain in-flight payloads.
                discard_attachments(vm, &s);
                vm.set_status(
                    match status {
                        MessageDelivery::Delivered => "Message delivered.",
                        MessageDelivery::Sent => "Message sent.",
                        _ => "Message submitted to the carrier.",
                    }
                    .into(),
                );
                refresh(vm, ui);
            }
        }
    }
}
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    if state.local.get() {
        return local_view(vm, ui);
    }
    if state.composing.get() {
        return compose_view(vm, ui);
    }
    if let Some(id) = state.thread.get() {
        return thread(vm, ui, id);
    }
    let native = state.local;
    let local = state.local;
    let query = state.query.get().to_lowercase();
    let store = vm.device_state();
    let allowed = store.snapshot.supports(DeviceOperation::ReadMessages);
    let rows = store
        .threads
        .into_iter()
        .filter(|v| {
            format!("{} {} {}", v.title, v.recipients.join(" "), v.snippet)
                .to_lowercase()
                .contains(&query)
        })
        .collect::<Vec<_>>();
    let list_model = vm.clone();
    let list_session = ui.clone();
    let list_query = state.query;
    let list = rerender(move || {
        let query = list_query.get().to_lowercase();
        let rows = list_model
            .device_state()
            .threads
            .into_iter()
            .filter(|v| {
                format!("{} {} {}", v.title, v.recipients.join(" "), v.snippet)
                    .to_lowercase()
                    .contains(&query)
            })
            .collect::<Vec<_>>();
        let model = list_model.clone();
        let session = list_session.clone();
        device::list(
            "DeviceMessageThreads",
            rows,
            Rc::new(move |item: DeviceMessageThread| {
                let open = model.clone();
                let ui = session.clone();
                let id = item.id.clone();
                let title = if item.title.is_empty() {
                    item.recipients.join(", ")
                } else {
                    item.title.clone()
                };
                rsx! {Button{class:"phone-flat",min_height:82.0,padding:(20.0,10.0),horizontal_content_alignment:H::Stretch,
                    automation_properties_name:format!("Device conversation {title}"),
                    on_click:move |_|{ui.state::<State>().thread.set(Some(id.clone()));open.request_device(DeviceRequest::Messages{thread_id:id.clone(),limit:200});if open.device_state().snapshot.role(DeviceRole::Sms)==HostRole::Active{open.request_device(DeviceRequest::MarkMessagesRead{thread_id:id.clone()});}},
                    StackPanel{spacing:4.0,{text(&title,26.0)}{muted(&item.snippet,17.0)}
                        {muted(&format!("{}{}",device::timestamp(&model,item.timestamp),if item.unread>0{format!(" · {} unread",item.unread)}else{String::new()}),13.0)}
                    }
                }}
            }),
        )
    });
    let new_vm = vm.clone();
    let new_ui = ui.clone();
    let refresh_vm = vm.clone();
    let refresh_ui = ui.clone();
    let tools = vm.clone();
    page(
        "messaging",
        rsx! {Grid{row_definitions:"Auto,Auto,*",
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0,20.0,12.0),{pivot("device",true,move||native.set(false))}{pivot("local",false,move||local.set(true))}}
            TextBox{Grid.row:1,class:"phone-input",margin:(20.0,0.0,20.0,10.0),name:"NativeMessageSearch",text:state.query.two_way(),placeholder_text:"search messages",automation_properties_name:"Search device messages"}
            Border{Grid.row:2,if allowed{if rows.is_empty(){{empty("No device messages","Refresh to read your messages, or create a new message.")}}else{{list}}}else{
                {scroll(rsx!{StackPanel{margin:20.0,spacing:18.0,{device::role(vm.clone(),DeviceRole::Sms)}{device::permission(vm.clone(),DevicePermission::ReadSms,"messages")}{muted("New messages can also open in the system composer.",17.0)}}})}
            }}
        }},
        app_bar(
            vec![
                action(Icon::Plus, "new", move || {
                    compose(&new_vm, &new_ui, Vec::new(), String::new())
                }),
                action(Icon::Refresh, "refresh", move || {
                    refresh(&refresh_vm, &refresh_ui)
                }),
                action(Icon::Settings, "settings", move || {
                    tools.navigate(phone_domain::Route::Setting("phone+messages".into()))
                }),
            ],
            false,
            vec![],
        ),
    )
}
fn local_view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let back = ui.state::<State>().local;
    rsx! {Grid{row_definitions:"Auto,*",Button{class:"phone-flat",content:"device messages",on_click:move |_|back.set(false)},Border{Grid.row:1,{crate::list::view(vm,ui)}}}}
}
fn compose_view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    let count = state.body.get().chars().count();
    let send_vm = vm.clone();
    let send_ui = ui.clone();
    let cancel = state.composing;
    let snapshot = vm.device_state().snapshot;
    let mms = state.multimedia.get();
    let direct = snapshot.supports(if mms {
        DeviceOperation::SendMultimediaMessages
    } else {
        DeviceOperation::SendMessages
    });
    let mode = state.multimedia;
    let pick = vm.clone();
    let pick_ui = ui.clone();
    let pending = state.pending.get().is_some();
    let mut attachments = stack_panel().spacing(8.0);
    for item in state.attachments.get() {
        let model = vm.clone();
        let chosen = state.attachments;
        let id = item.id.clone();
        attachments = attachments
            .child(text(
                &format!(
                    "{} · {}",
                    item.name,
                    item.bytes
                        .map_or_else(|| item.mime.clone(), |v| format!("{} KB", v.div_ceil(1024)))
                ),
                17.0,
            ))
            .child(flat("remove attachment", move || {
                if pending {
                    return;
                }
                chosen.update(|v| v.retain(|v| v.id != id));
                model.request_device(DeviceRequest::DiscardMessageAttachments {
                    ids: vec![id.clone()],
                });
            }));
    }
    let mut choices = stack_panel().spacing(5.0);
    if direct {
        let default = state.subscription;
        choices = choices.child(flat("use default SIM", move || default.set(None)));
        for account in vm.device_state().snapshot.sms_subscriptions {
            let selected = state.subscription;
            let current = selected.get().as_deref() == Some(&account.id);
            choices = choices.child(flat(
                &format!("{}{}", if current { "✓ " } else { "" }, account.label),
                move || selected.set(Some(account.id.clone())),
            ));
        }
    }
    page(
        "messaging",
        scroll(rsx! {StackPanel{margin:20.0,spacing:14.0,
            {text("new message",36.0)}{muted("to (separate numbers with commas)",16.0)}
            TextBox{class:"phone-input",name:"DeviceMessageRecipients",is_enabled:!pending,text:state.to.two_way(),placeholder_text:"phone numbers",automation_properties_name:"Device message recipients"}
            if snapshot.supports(DeviceOperation::SendMultimediaMessages)||snapshot.supports(DeviceOperation::PickMessageAttachments){
                {phone_controls::toggle("multimedia / group message",mms,move|value|if !pending{mode.set(value)})}
            }
            if mms{TextBox{class:"phone-input",name:"DeviceMessageSubject",is_enabled:!pending,text:state.subject.two_way(),placeholder_text:"subject (optional)",automation_properties_name:"Message subject"}}
            TextBox{class:"phone-input",name:"DeviceMessageBody",is_enabled:!pending,text:state.body.two_way(),accepts_return:true,min_height:160.0,text_wrapping:TextWrapping::Wrap,placeholder_text:"type a message",automation_properties_name:"Device message text"}
            {attachments}
            if snapshot.supports(DeviceOperation::PickMessageAttachments)&&!pending&&state.picker.get().is_none(){{flat("choose attachments",move||pick_attachments(&pick,&pick_ui))}}
            {muted(&format!("{count} characters"),15.0)}{choices}
            {muted(if direct&&mms{"Carrier charges and attachment limits apply. Group messages use MMS."}else if direct{"Carrier charges may apply. Multiple recipients receive separate SMS messages."}else{"The system composer confirms recipients and sends the message."},16.0)}
            if state.pending.get().is_some(){{muted("Submitting message…",17.0)}}
        }}),
        app_bar(
            vec![
                action(
                    Icon::Send,
                    if direct { "send" } else { "open composer" },
                    move || send(&send_vm, &send_ui),
                ),
                action(Icon::Close, "cancel", move || cancel.set(false)),
            ],
            false,
            vec![],
        ),
    )
}
fn send(vm: &PhoneViewModel, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    if state.pending.get_untracked().is_some() {
        return;
    }
    let recipients = state
        .to
        .get_untracked()
        .split([',', '\n'])
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let body = state.body.get_untracked();
    let subject = state.subject.get_untracked();
    let attachments = state
        .attachments
        .get_untracked()
        .into_iter()
        .map(|v| v.id)
        .collect::<Vec<_>>();
    let mms = state.multimedia.get_untracked() || !attachments.is_empty();
    let direct = vm.device_state().snapshot.supports(if mms {
        DeviceOperation::SendMultimediaMessages
    } else {
        DeviceOperation::SendMessages
    });
    let request = if direct {
        let Some(serial) = state.serial.get().checked_add(1) else {
            vm.set_status("Reopen Messaging to send another message.".into());
            return;
        };
        state.serial.set(serial);
        let client_id = format!(
            "phone-message:{}:{}:{serial}",
            vm.document_epoch(),
            vm.now()
        );
        let request = if mms {
            DeviceRequest::SendMultimediaMessage {
                recipients,
                subject,
                body,
                attachments,
                subscription: state.subscription.get_untracked(),
                client_id: client_id.clone(),
            }
        } else {
            DeviceRequest::SendMessage {
                recipients,
                body,
                subscription: state.subscription.get_untracked(),
                client_id: client_id.clone(),
            }
        };
        if let Err(error) = request.validate() {
            vm.set_status(error);
            return;
        }
        state.pending.set(Some(client_id));
        request
    } else {
        if mms {
            DeviceRequest::ComposeMultimediaMessage {
                recipients,
                subject,
                body,
                attachments,
            }
        } else {
            DeviceRequest::ComposeMessage { recipients, body }
        }
    };
    vm.request_device(request);
}
fn thread(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: String) -> Element {
    let store = vm.device_state();
    let accent = vm.data().settings.accent.clone();
    let record = store.threads.iter().find(|v| v.id == id);
    let title = record
        .map(|v| {
            if v.title.is_empty() {
                v.recipients.join(", ")
            } else {
                v.title.clone()
            }
        })
        .unwrap_or_else(|| "conversation".into());
    let messages = if store.message_thread.as_deref() == Some(&id) {
        store.messages
    } else {
        Vec::new()
    };
    let mut bubbles = stack_panel().spacing(14.0).margin((20.0, 0.0));
    for message in messages {
        let selected = ui.state::<State>().context;
        let key = message.id.clone();
        let selected_now = selected.get().as_deref() == Some(&message.id);
        let mut actions = stack_panel().spacing(4.0);
        if selected_now {
            if message.can_download
                && vm
                    .device_state()
                    .snapshot
                    .supports(DeviceOperation::DownloadMessages)
            {
                let model = vm.clone();
                let download_ui = ui.clone();
                let message_id = message.id.clone();
                let thread_id = id.clone();
                actions = actions.child(flat("download message", move || {
                    *download_ui.state::<State>().download.borrow_mut() = Some((
                        message_id.clone(),
                        thread_id.clone(),
                        model.device_state().download_revision,
                    ));
                    model.request_device(DeviceRequest::DownloadMessage {
                        message_id: message_id.clone(),
                    })
                }));
            }
            if message.attachment_count > 0
                && vm
                    .device_state()
                    .snapshot
                    .supports(DeviceOperation::MessageAttachments)
            {
                let model = vm.clone();
                let message_id = message.id.clone();
                actions = actions.child(flat("show attachments", move || {
                    model.request_device(DeviceRequest::MessageAttachments {
                        message_id: message_id.clone(),
                    })
                }));
                if store.attachment_message.as_deref() == Some(&message.id) {
                    for item in &store.message_attachments {
                        let model = vm.clone();
                        let message_id = message.id.clone();
                        let id = item.id.clone();
                        actions = actions.child(flat(&format!("open {}", item.name), move || {
                            model.request_device(DeviceRequest::OpenMessageAttachment {
                                message_id: message_id.clone(),
                                id: id.clone(),
                            })
                        }));
                    }
                }
            }
            let copy = vm.clone();
            let body = message.body.clone();
            actions = actions.child(flat("copy", move || {
                copy.request(phone_state::HostRequest::CopyText { text: body.clone() })
            }));
            let forward = vm.clone();
            let forward_ui = ui.clone();
            let body = message.body.clone();
            actions = actions.child(flat("forward", move || {
                compose(&forward, &forward_ui, Vec::new(), body.clone())
            }));
            if vm.device_state().snapshot.role(DeviceRole::Sms) == HostRole::Active {
                let remove = vm.clone();
                let confirm = ui.clone();
                let thread_id = id.clone();
                let message_id = message.id.clone();
                actions = actions.child(flat("delete", move || {
                    let model = remove.clone();
                    let thread_id = thread_id.clone();
                    let message_id = message_id.clone();
                    confirm.confirm(
                        "delete this device message?",
                        Rc::new(move || {
                            model.request_device(DeviceRequest::DeleteMessages {
                                thread_id: thread_id.clone(),
                                ids: vec![message_id.clone()],
                            })
                        }),
                    );
                }));
            }
        }
        bubbles=bubbles.child(rsx!{StackPanel{spacing:5.0,
            Button{class:"phone-flat",padding:12.0,horizontal_content_alignment:H::Stretch,background:if message.outgoing{accent.as_str()}else{"#343434"},
                automation_properties_name:format!("Message {}",message.id),on_click:move |_|selected.set(if selected.get_untracked().as_deref()==Some(&key){None}else{Some(key.clone())}),
                StackPanel{spacing:5.0,if !message.subject.is_empty(){TextBlock{text:message.subject,font_size:23.0,foreground:"White",text_wrapping:TextWrapping::Wrap}}TextBlock{text:message.body,font_size:21.0,foreground:"White",text_wrapping:TextWrapping::Wrap}
                    if message.attachment_count>0{TextBlock{text:format!("{} attachment(s)",message.attachment_count),font_size:15.0,foreground:"White"}}
                    TextBlock{text:format!("{} · {}",device::timestamp(&vm,message.timestamp),delivery(message.status)),font_size:12.0,foreground:"White"}
                }}{actions}
        }});
    }
    let reply_vm = vm.clone();
    let reply_ui = ui.clone();
    let recipients = record.map(|v| v.recipients.clone()).unwrap_or_default();
    let refresh_vm = vm.clone();
    let refresh_id = id;
    page(
        "messaging",
        rsx! {Grid{row_definitions:"Auto,*",Border{margin:(20.0,0.0,20.0,14.0),{text(&title,32.0)}},Border{Grid.row:1,{scroll(bubbles.into())}}}},
        app_bar(
            vec![
                action(Icon::Message, "reply", move || {
                    compose(&reply_vm, &reply_ui, recipients.clone(), String::new())
                }),
                action(Icon::Refresh, "refresh", move || {
                    refresh_vm.request_device(DeviceRequest::Messages {
                        thread_id: refresh_id.clone(),
                        limit: 200,
                    })
                }),
            ],
            false,
            vec![],
        ),
    )
}
fn discard_attachments(vm: &PhoneViewModel, state: &State) {
    state.picker.set(None);
    let ids = state
        .attachments
        .get_untracked()
        .into_iter()
        .map(|v| v.id)
        .collect::<Vec<_>>();
    if !ids.is_empty() {
        vm.request_device(DeviceRequest::DiscardMessageAttachments { ids });
    }
    state.attachments.set(Vec::new());
    state
        .attachment_revision
        .set(vm.device_state().draft_attachment_revision);
}
fn pick_attachments(vm: &PhoneViewModel, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    if state.picker.get_untracked().is_some() || state.pending.get_untracked().is_some() {
        return;
    }
    let Some(serial) = state.serial.get().checked_add(1) else {
        vm.set_status("Attachment request identities exhausted.".into());
        return;
    };
    state.serial.set(serial);
    let client_id = format!("attachments-{}-{}-{serial}", vm.document_epoch(), vm.now());
    state.picker.set(Some(client_id.clone()));
    vm.request_device(DeviceRequest::PickMessageAttachments { client_id });
}
fn delivery(status: MessageDelivery) -> &'static str {
    match status {
        MessageDelivery::Unknown => "status unavailable",
        MessageDelivery::Draft => "draft",
        MessageDelivery::Queued => "queued",
        MessageDelivery::Sending => "sending",
        MessageDelivery::Sent => "sent",
        MessageDelivery::Delivered => "delivered",
        MessageDelivery::Failed => "failed",
        MessageDelivery::Received => "received",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Rc<PhoneViewModel>, Rc<Session>) {
        let vm = PhoneViewModel::new(phone_domain::PhoneData::demo(), Rc::new(|_| {}));
        let registry = Rc::new(phone_sdk::AppRegistry::new(vec![crate::registration()]).unwrap());
        (vm, Rc::new(Session::new(0, registry)))
    }
    fn attachment() -> DeviceAttachment {
        DeviceAttachment {
            id: "draft-attachment:fixture".into(),
            name: "photo.png".into(),
            mime: "image/png".into(),
            bytes: Some(10),
        }
    }
    #[test]
    fn downloaded_message_follows_only_its_own_still_selected_conversation() {
        let (vm, ui) = fixture();
        vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot {
            operations: vec![DeviceOperation::ReadMessages],
            ..Default::default()
        }))
        .unwrap();
        let state = ui.state::<State>();
        state.thread.set(Some("pending-thread".into()));
        *state.download.borrow_mut() = Some(("message-a".into(), "pending-thread".into(), 0));
        vm.set_device_response(DeviceResponse::MessageDownloaded {
            message_id: "message-a".into(),
            thread_id: "resolved-thread".into(),
        })
        .unwrap();
        tick(&vm, &ui);
        assert_eq!(
            state.thread.get_untracked().as_deref(),
            Some("resolved-thread")
        );
        assert!(state.download.borrow().is_none());

        *state.download.borrow_mut() = Some((
            "message-b".into(),
            "resolved-thread".into(),
            vm.device_state().download_revision,
        ));
        state.thread.set(Some("another-conversation".into()));
        vm.set_device_response(DeviceResponse::MessageDownloaded {
            message_id: "message-b".into(),
            thread_id: "resolved-b".into(),
        })
        .unwrap();
        tick(&vm, &ui);
        assert_eq!(
            state.thread.get_untracked().as_deref(),
            Some("another-conversation")
        );
        assert!(state.download.borrow().is_none());
        vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot::default()))
            .unwrap();
        assert!(vm.device_state().download_results.is_empty());
    }
    #[test]
    fn late_picker_does_not_replace_a_new_message_draft() {
        let (vm, ui) = fixture();
        let state = ui.state::<State>();
        state.picker.set(Some("old-picker".into()));
        compose(&vm, &ui, vec!["123".into()], "new draft".into());
        state.picker.set(Some("new-picker".into()));
        vm.set_device_response(DeviceResponse::DraftAttachments {
            client_id: "old-picker".into(),
            attachments: Some(vec![attachment()]),
            error: None,
        })
        .unwrap();
        tick(&vm, &ui);
        assert!(state.attachments.get_untracked().is_empty());
        assert_eq!(state.picker.get_untracked().as_deref(), Some("new-picker"));
        assert_eq!(state.body.get_untracked(), "new draft");
        vm.set_device_response(DeviceResponse::DraftAttachments {
            client_id: "new-picker".into(),
            attachments: Some(vec![attachment()]),
            error: None,
        })
        .unwrap();
        tick(&vm, &ui);
        assert_eq!(state.attachments.get_untracked().len(), 1);
        assert!(state.picker.get_untracked().is_none());
    }
    #[test]
    fn picker_cancel_and_older_send_receipt_preserve_the_current_draft() {
        let (vm, ui) = fixture();
        let state = ui.state::<State>();
        state.pending.set(Some("old-send".into()));
        compose(&vm, &ui, vec!["123".into()], "current text".into());
        state.attachments.set(vec![attachment()]);
        state.picker.set(Some("cancelled-picker".into()));
        vm.set_device_response(DeviceResponse::DraftAttachments {
            client_id: "cancelled-picker".into(),
            attachments: None,
            error: None,
        })
        .unwrap();
        vm.set_device_response(DeviceResponse::MessageSent {
            client_id: "old-send".into(),
            id: "native-message".into(),
            status: MessageDelivery::Sent,
        })
        .unwrap();
        tick(&vm, &ui);
        assert_eq!(state.body.get_untracked(), "current text");
        assert_eq!(state.attachments.get_untracked().len(), 1);
        assert!(state.picker.get_untracked().is_none());
    }
}
