//! Shared native app-bar entry points for personal-information file exchange.
use crate::session::Session;
use ferroui_view::prelude::Element;
use phone_controls::flat;
use phone_domain::Id;
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;

pub(super) fn contacts(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    contact: Option<Id>,
) -> Vec<Element> {
    let save = vm.clone();
    let save_ui = ui.clone();
    let mut commands = vec![flat(
        if contact.is_some() {
            "save contact copy"
        } else {
            "save contacts copy"
        },
        move || {
            save_ui.menu.set(false);
            let ids = contact.map(|id| vec![id]).unwrap_or_else(|| {
                save.data()
                    .contacts
                    .iter()
                    .map(|contact| contact.id)
                    .collect()
            });
            save.request(HostRequest::ExportContacts { ids });
        },
    )];
    if let Some(id) = contact {
        commands.insert(
            0,
            flat("share contact", move || {
                ui.menu.set(false);
                vm.request(HostRequest::ShareContacts { ids: vec![id] });
            }),
        );
    } else {
        commands.insert(
            0,
            flat("import contacts", move || {
                ui.menu.set(false);
                vm.request(HostRequest::ImportContacts);
            }),
        );
    }
    commands
}

pub(super) fn calendar(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    event: Option<(Id, bool)>,
) -> Vec<Element> {
    let save = vm.clone();
    let save_ui = ui.clone();
    let save_label = match event {
        Some((_, true)) => "save series copy",
        Some(_) => "save appointment copy",
        None => "save calendar copy",
    };
    let mut commands = vec![flat(save_label, move || {
        save_ui.menu.set(false);
        let ids = event
            .map(|(id, _)| vec![id])
            .unwrap_or_else(|| save.data().events.iter().map(|event| event.id).collect());
        save.request(HostRequest::ExportCalendar { ids });
    })];
    if let Some((id, repeat)) = event {
        commands.insert(
            0,
            flat(
                if repeat {
                    "share series"
                } else {
                    "share appointment"
                },
                move || {
                    ui.menu.set(false);
                    vm.request(HostRequest::ShareCalendar { ids: vec![id] });
                },
            ),
        );
    } else {
        commands.insert(
            0,
            flat("import appointments", move || {
                ui.menu.set(false);
                vm.request(HostRequest::ImportCalendar);
            }),
        );
    }
    commands
}
