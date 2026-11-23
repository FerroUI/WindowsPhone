use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_base::media::TextWrapping;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_domain::{AppId, Contact, Conversation, Route};
use phone_sdk::{
    common::{muted, page},
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, conversation: Conversation) -> Element {
    let name = vm.data().contact_name(&conversation);
    let id = conversation.id;
    let bubbles = super::bubbles::list(&vm, &ui, &conversation);
    let active = ui
        .state::<crate::MessageState>()
        .context
        .get()
        .and_then(|message| {
            conversation
                .messages
                .iter()
                .find(|value| value.id == message)
                .cloned()
        });
    let person = vm.clone();
    let person_ui = ui.clone();
    let person_thread = conversation.clone();
    let body = rsx! {Grid{row_definitions:"Auto,*,Auto,Auto",
        Button{class:"phone-flat",padding:(20.0,0.0,20.0,12.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
            automation_properties_name:format!("Contact details for {name}"),on_click:move |_|contact(person.clone(),person_ui.clone(),person_thread.clone()),
            StackPanel{TextBlock{text:name.clone(),font_size:34.0,text_wrapping:TextWrapping::Wrap}{muted("text",14.0)}}
        }
        ScrollViewer{Grid.row:1,key:format!("conversation-{id}-{}",conversation.messages.len()),name:format!("ConversationMessages-{id}"),
            horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
            on_loaded:move |event|if let Some(scroll)=event.source().and_then(|source|source.cast::<ScrollViewer>()){scroll.scroll_to_end();},
            StackPanel{margin:(20.0,0.0),for bubble in bubbles{{bubble}}}
        }
        if let Some(message)=active {Border{Grid.row:2,{super::bubbles::context(vm.clone(),ui.clone(),id,message)}}}
        Border{Grid.row:3,{composer(vm.clone(),ui.clone(),id)}}
    }};
    let call = vm.clone();
    let number = conversation.recipient.clone();
    let call_name = name;
    let remove = vm.clone();
    let confirm = ui.clone();
    let menu = ui.menu;
    let person_vm = vm.clone();
    let person_state = ui.clone();
    let person_thread = conversation;
    let settings = vm.clone();
    page(
        "messaging",
        body,
        app_bar(
            vec![
                action(Icon::Phone, "call", move || {
                    call.record_call(call_name.clone(), number.clone())
                }),
                action(Icon::People, "contact", move || {
                    contact(
                        person_vm.clone(),
                        person_state.clone(),
                        person_thread.clone(),
                    )
                }),
                action(Icon::More, "more", move || {
                    menu.update(|value| *value = !*value)
                }),
            ],
            ui.menu.get(),
            vec![
                flat("delete thread", move || {
                    let model = remove.clone();
                    let drafts = confirm.state::<crate::MessageState>().drafts;
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
                }),
                flat("notification settings", move || {
                    settings.navigate(Route::Setting("notification:Messaging".into()))
                }),
            ],
        ),
    )
}

fn composer(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: u64) -> Element {
    let drafts = ui.state::<crate::MessageState>().drafts;
    let draft = drafts.get().get(&id).cloned().unwrap_or_default();
    let count = draft.chars().count();
    let send_vm = vm.clone();
    let send_ui = ui.clone();
    rsx! {StackPanel{margin:(14.0,8.0),spacing:3.0,
        Grid{column_definitions:"*,Auto",column_spacing:6.0,
            TextBox{key:format!("composer-{id}"),name:format!("ConversationInput-{id}"),class:"phone-input",text:draft,placeholder_text:"type a message",automation_properties_name:"Message",accepts_return:true,text_wrapping:TextWrapping::Wrap,min_height:50.0,max_height:130.0,
                on_text_changed:move |event|if let Some(input)=event.source().and_then(|source|source.cast::<TextBox>()){
                    let text=input.text().unwrap_or_default();
                    if drafts.get_untracked().get(&id).map(String::as_str).unwrap_or("")!=text {
                        drafts.update(|values|{if text.is_empty(){values.remove(&id);}else{values.insert(id,text);}});
                    }
                }
            }
            Border{Grid.column:1,{action(Icon::Send,"send",move ||{
                let body=send_ui.state::<crate::MessageState>().drafts.get_untracked().get(&id).cloned().unwrap_or_default();
                if !body.trim().is_empty(){send_vm.send_message(id,body);send_ui.state::<crate::MessageState>().drafts.update(|values|{values.remove(&id);});}
            })}}
        }
        if count>0{TextBlock{text:format!("{count} characters"),font_size:12.0,horizontal_alignment:H::Right,foreground:phone_controls::secondary_color()}}
    }}
}

fn contact(vm: Rc<PhoneViewModel>, ui: Rc<Session>, conversation: Conversation) {
    if let Some(id) = conversation
        .contact_id
        .filter(|id| vm.data().contact(*id).is_some())
    {
        vm.navigate(Route::Contact(id));
        return;
    }
    let error = ui.form_error;
    let number = conversation.recipient.clone();
    let conversation_id = conversation.id;
    ui.form(
        "save number",
        vec![("name", String::new()), ("phone", number)],
        Rc::new(move |values| {
            let name = values
                .first()
                .cloned()
                .unwrap_or_default()
                .trim()
                .to_owned();
            let phone = values.get(1).cloned().unwrap_or_default().trim().to_owned();
            if name.is_empty() || phone.is_empty() {
                error.set("Enter a name and phone number.".into());
                return;
            }
            if name.chars().count() > 256 {
                error.set("Use a contact name with at most 256 characters.".into());
                return;
            }
            vm.save_contact(Contact {
                id: 0,
                name: name.clone(),
                phone: phone.clone(),
                ..Default::default()
            });
            let contact = vm
                .data()
                .contacts
                .iter()
                .filter(|contact| contact.name == name && contact.phone == phone)
                .max_by_key(|contact| contact.id)
                .map(|contact| contact.id);
            if let Some(contact) = contact {
                let value = vm.data().conversation(conversation_id).cloned();
                if let Some(mut value) = value {
                    value.contact_id = Some(contact);
                    vm.save_conversation(value);
                }
            } else {
                error.set(vm.status());
            }
        }),
    );
}
