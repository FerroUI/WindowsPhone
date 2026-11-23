use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use phone_controls::flat;
use phone_domain::{Action, Conversation, Message};
use phone_sdk::{
    common::{muted, text},
    session::Session,
};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn list(
    vm: &Rc<PhoneViewModel>,
    ui: &Rc<Session>,
    conversation: &Conversation,
) -> Vec<Element> {
    let settings = vm.data().settings.clone();
    let offset = i64::from(vm.utc_offset());
    let mut date = None;
    let mut elements = Vec::new();
    for message in &conversation.messages {
        let local = message.timestamp.saturating_add(offset);
        let day = phone_domain::date_parts(local);
        if date != Some(day) {
            elements.push(rsx!{Border{margin:(0.0,18.0,0.0,6.0),{muted(&settings.date_order.format(day.0,day.1,day.2),15.0)}}});
            date = Some(day);
        }
        let time = phone_domain::time_text(local, settings.time_24h);
        let caption = if message.draft {
            format!("{time} · draft opened in messaging app")
        } else {
            time
        };
        let fill = if settings.high_contrast {
            "#000000".into()
        } else if message.incoming {
            if settings.dark {
                "#333333".into()
            } else {
                "#DDDDDD".into()
            }
        } else {
            settings.accent.clone()
        };
        let foreground = if message.incoming {
            settings.foreground()
        } else {
            "White"
        };
        let id = message.id;
        let context = ui.state::<crate::MessageState>().context;
        elements.push(rsx!{Border{key:format!("message-{id}"),name:format!("Context-Message-{id}"),background:fill.as_str(),
            border_brush:if settings.high_contrast{"White"}else{"Transparent"},border_thickness:if settings.high_contrast{1.0}else{0.0},
            margin:if message.incoming{(0.0,6.0,32.0,6.0)}else{(32.0,6.0,0.0,6.0)},padding:14.0,
            automation_properties_name:format!("{}: {}",if message.incoming{"Received message"}else{"Outgoing draft"},message.text),
            on_context_requested:move |event|{event.set_handled(true);context.set(Some(id));},
            StackPanel{spacing:8.0,
                TextBlock{text:message.text.clone(),font_size:21.0*phone_controls::text_scale(),text_wrapping:TextWrapping::Wrap,foreground:foreground}
                TextBlock{text:caption,font_size:12.0,text_wrapping:TextWrapping::Wrap,foreground:if settings.high_contrast{"White"}else if message.incoming{settings.secondary()}else{"#EEEEEE"}}
            }
        }});
    }
    elements
}

pub(super) fn context(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    conversation: u64,
    message: Message,
) -> Element {
    let copy_vm = vm.clone();
    let copy_ui = ui.clone();
    let copy_text = message.text.clone();
    let forward_vm = vm.clone();
    let forward_ui = ui.clone();
    let forward_text = message.text.clone();
    let remove_vm = vm.clone();
    let remove_ui = ui.clone();
    let message_id = message.id;
    let close = ui.state::<crate::MessageState>().context;
    rsx! {Border{background:if vm.data().settings.high_contrast{"#000000"}else if vm.data().settings.dark{"#1F1F1F"}else{"#E7E7E7"},
        border_brush:if vm.data().settings.high_contrast{"White"}else{"Transparent"},border_thickness:if vm.data().settings.high_contrast{1.0}else{0.0},padding:(20.0,12.0),
        StackPanel{spacing:3.0,
            {text("message",27.0)}
            {flat("copy",move ||{copy_vm.request(HostRequest::CopyText{text:copy_text.clone()});copy_ui.state::<crate::MessageState>().context.set(None);})}
            {flat("forward",move ||{forward_ui.state::<crate::MessageState>().context.set(None);super::compose_body(forward_vm.clone(),forward_ui.clone(),String::new(),forward_text.clone());})}
            {flat("delete",move ||{
                let model=remove_vm.clone();let state=remove_ui.clone();
                remove_ui.confirm("delete this message?",Rc::new(move ||{model.dispatch(Action::DeleteMessage{conversation,message:message_id});state.state::<crate::MessageState>().context.set(None);}));
            })}
            {flat("cancel",move ||close.set(None))}
        }
    }}
}
