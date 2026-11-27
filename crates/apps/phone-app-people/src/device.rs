use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar, flat};
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
pub fn refresh(vm: &PhoneViewModel, ui: &Session) {
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
pub(super) fn selected_detail(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Option<Element> {
    let state = ui.state::<State>();
    let id = state.selected.get()?;
    let contact = vm
        .device_state()
        .contacts
        .into_iter()
        .find(|contact| contact.id == id);
    if let Some(contact) = contact {
        Some(detail(vm, ui, contact))
    } else {
        state.selected.set(None);
        None
    }
}

/// Finite pane content only: Panorama owns its title/section heading and search
/// owns its editor. Device contacts remain in the host store, outside backups.
pub(super) fn pane(vm: Rc<PhoneViewModel>, ui: Rc<Session>, searching: bool) -> Element {
    let model = vm.clone();
    let session = ui.clone();
    let list = rerender(move || {
        let query = if searching {
            session.state::<State>().query.get().trim().to_lowercase()
        } else {
            String::new()
        };
        let memory = session.state::<crate::hub::State>();
        if searching {
            memory.query_changed(crate::hub::SearchScope::Device, &query);
        }
        let retained = if searching {
            &memory.device_search
        } else {
            &memory.device
        };
        let rows = model
            .device_state()
            .contacts
            .into_iter()
            .filter(|contact| {
                format!(
                    "{} {} {}",
                    contact.name,
                    contact.phones.join(" "),
                    contact.emails.join(" ")
                )
                .to_lowercase()
                .contains(&query)
            })
            .collect::<Vec<_>>();
        let is_empty = rows.is_empty();
        let session = session.clone();
        let contact_vm = model.clone();
        let epoch = model.document_epoch();
        let list = device::grouped_list(
            if searching {
                "DevicePeopleSearchList"
            } else {
                "DeviceContacts"
            },
            rows,
            Rc::new(move |contact: DeviceContact| {
                let selection = session.state::<State>().selected;
                let model = contact_vm.clone();
                let title = if contact.name.is_empty() {
                    contact
                        .phones
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "contact".into())
                } else {
                    contact.name.clone()
                };
                rsx! {Button {class:"phone-flat",min_height:70.0,padding:(20.0,10.0),horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,
                automation_properties_name:format!("Device contact {title}"),
                on_click:move |_|if model.document_epoch() == epoch {selection.set(Some(contact.id.clone()));},
                StackPanel {spacing:4.0,{text(&title,26.0)}{muted(contact.phones.first().map(String::as_str).unwrap_or(""),16.0)}}}}
            }),
            retained.node,
            retained.scroll.clone(),
        );
        // Keep the retained native list hooks stable across an empty provider
        // observation or search result; only its visible content changes.
        if is_empty {
            empty(
                if query.is_empty() {
                    "No device contacts"
                } else {
                    "No contacts"
                },
                if query.is_empty() {
                    "Refresh to read contacts allowed by the device."
                } else {
                    "Try a different name or number."
                },
            )
        } else {
            list
        }
    });
    let allowed = matches!(
        vm.device_state()
            .snapshot
            .permission(DevicePermission::Contacts),
        PermissionState::Granted | PermissionState::Limited
    );
    rsx! {Border {name:if searching {"DevicePeopleSearchResults"} else {"DevicePeoplePane"},
        if allowed {{list}} else {{scroll(rsx!{StackPanel{margin:20.0,spacing:18.0,
            {device::permission(vm.clone(),DevicePermission::Contacts,"contacts")}
            {muted("Device contacts are read from your address book and stay outside Windows Phone backups.",17.0)}
        }})}}
    }}
}

pub(super) fn actions(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Vec<Element> {
    let new_vm = vm.clone();
    let new_ui = ui.clone();
    vec![
        action(Icon::Plus, "new on device", move || {
            edit(new_vm.clone(), new_ui.clone(), DeviceContact::default())
        }),
        action(Icon::Refresh, "refresh", move || refresh(&vm, &ui)),
    ]
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
