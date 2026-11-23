use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, pivot, Icon};
use phone_sdk::{
    common::{empty, muted, page, scroll, text},
    device,
    host::platform::*,
    session::Session,
};
use phone_state::PhoneViewModel;
use std::rc::Rc;
pub struct State {
    pub available: std::cell::Cell<bool>,
    readable: std::cell::Cell<bool>,
    pub selected: Signal<Option<String>>,
    pub query: Signal<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            available: std::cell::Cell::new(false),
            readable: std::cell::Cell::new(false),
            selected: signal(None),
            query: signal(String::new()),
        }
    }
}
pub fn tick(vm: &PhoneViewModel, ui: &Rc<Session>) {
    let state = ui.state::<State>();
    let readable = vm
        .device_state()
        .snapshot
        .supports(DeviceOperation::ReadContacts);
    if readable != state.readable.replace(readable) && readable {
        refresh(vm, ui);
    }
}
pub fn refresh(vm: &PhoneViewModel, ui: &Rc<Session>) {
    let available = vm.platform_state().supports(HostFeature::Contacts);
    ui.state::<State>().available.set(available);
    if !available {
        return;
    }
    vm.refresh_device();
    if matches!(
        vm.device_state()
            .snapshot
            .permission(DevicePermission::Contacts),
        PermissionState::Granted | PermissionState::Limited
    ) {
        vm.request_device(DeviceRequest::Contacts {
            query: ui.state::<State>().query.get_untracked(),
            limit: 1000,
        });
    }
}
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let state = ui.state::<State>();
    if let Some(contact) = state
        .selected
        .get()
        .and_then(|id| vm.device_state().contacts.into_iter().find(|v| v.id == id))
    {
        return detail(vm, ui, contact);
    }
    let all = ui.pivot;
    let favorites = ui.pivot;
    let native = ui.pivot;
    let model = vm.clone();
    let session = ui.clone();
    let list = rerender(move || {
        let query = session.state::<State>().query.get().to_lowercase();
        let rows = model
            .device_state()
            .contacts
            .into_iter()
            .filter(|v| {
                format!("{} {} {}", v.name, v.phones.join(" "), v.emails.join(" "))
                    .to_lowercase()
                    .contains(&query)
            })
            .collect::<Vec<_>>();
        let session = session.clone();
        device::list(
            "DeviceContacts",
            rows,
            Rc::new(move |contact: DeviceContact| {
                let selection = session.state::<State>().selected;
                let title = if contact.name.is_empty() {
                    contact
                        .phones
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "contact".into())
                } else {
                    contact.name.clone()
                };
                rsx! {Button{class:"phone-flat",min_height:70.0,padding:(20.0,10.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                automation_properties_name:format!("Device contact {title}"),on_click:move |_|selection.set(Some(contact.id.clone())),
                StackPanel{spacing:4.0,{text(&title,26.0)}{muted(contact.phones.first().map(String::as_str).unwrap_or(""),16.0)}}}}
            }),
        )
    });
    let allowed = matches!(
        vm.device_state()
            .snapshot
            .permission(DevicePermission::Contacts),
        PermissionState::Granted | PermissionState::Limited
    );
    let new_vm = vm.clone();
    let new_ui = ui.clone();
    let refresh_vm = vm.clone();
    let refresh_ui = ui.clone();
    page(
        "people",
        rsx! {Grid{row_definitions:"Auto,Auto,*",
            StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0,20.0,12.0),{pivot("all",false,move||all.set(0))}{pivot("favorites",false,move||favorites.set(1))}{pivot("device",true,move||native.set(2))}}
            TextBox{Grid.row:1,class:"phone-input",name:"DevicePeopleSearch",text:state.query.two_way(),margin:(20.0,0.0,20.0,10.0),placeholder_text:"search device contacts",automation_properties_name:"Search device contacts"}
            Border{Grid.row:2,if allowed{if vm.device_state().contacts.is_empty(){{empty("No device contacts","Refresh to read contacts allowed by the device.")}}else{{list}}}else{{scroll(rsx!{StackPanel{margin:20.0,spacing:18.0,{device::permission(vm.clone(),DevicePermission::Contacts,"contacts")}{muted("Device contacts are read from your address book and stay outside Windows Phone backups.",17.0)}}})}}}
        }},
        app_bar(
            vec![
                action(Icon::Plus, "new on device", move || {
                    edit(new_vm.clone(), new_ui.clone(), DeviceContact::default())
                }),
                action(Icon::Refresh, "refresh", move || {
                    refresh(&refresh_vm, &refresh_ui)
                }),
            ],
            false,
            vec![],
        ),
    )
}
fn detail(vm: Rc<PhoneViewModel>, ui: Rc<Session>, contact: DeviceContact) -> Element {
    let mut rows = stack_panel()
        .margin(20.0)
        .spacing(18.0)
        .child(text(&contact.name, 40.0));
    for number in &contact.phones {
        let call = vm.clone();
        let name = contact.name.clone();
        let number = number.clone();
        let dial = number.clone();
        let message = vm.clone();
        let message_ui = ui.clone();
        rows = rows
            .child(text(&number, 25.0))
            .child(flat("call", move || {
                call.record_call(name.clone(), dial.clone())
            }))
            .child(flat("text", move || {
                phone_app_messaging::compose(message.clone(), message_ui.clone(), number.clone())
            }));
    }
    for address in &contact.emails {
        let model = vm.clone();
        let email = address.clone();
        rows = rows.child(flat(address, move || {
            model.request(phone_state::HostRequest::Email {
                to: email.clone(),
                subject: String::new(),
                body: String::new(),
            })
        }));
    }
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let editable = contact.clone();
    let copy = vm.clone();
    let copied = contact.clone();
    let remove = vm.clone();
    let confirm = ui.clone();
    let id = contact.id.clone();
    let menu = ui.menu;
    let mut commands = vec![flat("copy to local contacts", move || {
        copy.save_contact(phone_domain::Contact {
            id: 0,
            name: copied.name.clone(),
            phone: copied.phones.first().cloned().unwrap_or_default(),
            email: copied.emails.first().cloned().unwrap_or_default(),
            favorite: copied.favorite,
            ..Default::default()
        })
    })];
    if vm
        .device_state()
        .snapshot
        .supports(DeviceOperation::WriteContacts)
    {
        commands.push(flat("delete from device", move || {
            let model = remove.clone();
            let id = id.clone();
            confirm.confirm(
                "delete this device contact?",
                Rc::new(move || {
                    model.request_device(DeviceRequest::DeleteContact { id: id.clone() })
                }),
            );
        }));
    }
    page(
        "people",
        scroll(rows.into()),
        app_bar(
            vec![
                action(Icon::Edit, "edit on device", move || {
                    edit(edit_vm.clone(), edit_ui.clone(), editable.clone())
                }),
                action(Icon::More, "more", move || menu.set(!menu.get_untracked())),
            ],
            ui.menu.get(),
            commands,
        ),
    )
}
fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, contact: DeviceContact) {
    if !vm
        .device_state()
        .snapshot
        .supports(DeviceOperation::WriteContacts)
    {
        vm.set_status(
            "Device contact editing is unavailable. Allow contacts access in Settings.".into(),
        );
        return;
    }
    let error = ui.form_error;
    let epoch = vm.document_epoch();
    ui.form_with_action(
        if contact.id.is_empty() {
            "new device contact"
        } else {
            "edit device contact"
        },
        vec![
            ("name", contact.name.clone()),
            ("phone numbers", contact.phones.join(", ")),
            ("email addresses", contact.emails.join(", ")),
        ],
        "save on device",
        Rc::new(move |values| {
            if vm.document_epoch() != epoch {
                error.set("The phone data changed. Reopen the contact.".into());
                return;
            }
            let field = |i: usize| values.get(i).cloned().unwrap_or_default();
            let split = |value: String| {
                value
                    .split([',', '\n'])
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            };
            let request = DeviceRequest::SaveContact {
                contact: DeviceContact {
                    id: contact.id.clone(),
                    name: field(0).trim().into(),
                    phones: split(field(1)),
                    emails: split(field(2)),
                    favorite: contact.favorite,
                },
            };
            if let Err(message) = request.validate() {
                error.set(message);
                return;
            }
            vm.request_device(request);
        }),
    );
}
