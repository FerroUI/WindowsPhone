use crate::{commands, notebooks::model::Library, state::NoteSession};
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, Icon};
use phone_domain::{Note, Route};
use phone_sdk::{common::*, session::Session};
use phone_state::{HostRequest, PhoneViewModel};
use std::rc::Rc;
pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, note: Note, library: &Library) -> Element {
    let id = note.id;
    let path = library
        .section(library.section_for(id))
        .map(|(notebook, section)| format!("{} / {}", notebook.name, section.name))
        .unwrap_or_default();
    let progress = crate::checklist::summary(&note.body)
        .map(|(done, total)| format!("{done} of {total} complete"))
        .unwrap_or_else(|| "Saved on this device".into());
    let body = rsx! {Grid{row_definitions:"Auto,*,Auto",
    StackPanel{margin:(20.0,12.0),spacing:8.0,{muted(&path,14.0)}{text(&note.title,38.0)}}
    Border{Grid.row:1,{crate::body::view(note.clone(),vm.clone())}}
    Border{Grid.row:2,margin:(20.0,8.0),{muted(&progress,14.0)}}}};
    let (edit_vm, edit_ui, editable) = (vm.clone(), ui.clone(), note.clone());
    let (todo_vm, todo_ui) = (vm.clone(), ui.clone());
    let more = ui.clone();
    let moving = ui.clone();
    let pin = vm.clone();
    let (delete_vm, delete_ui) = (vm.clone(), ui.clone());
    let share_vm = vm.clone();
    let share_note = note.clone();
    let copy_vm = vm.clone();
    let copy_note = note;
    let menu = vec![
        flat("move page", move || {
            moving.menu.set(false);
            moving.state::<NoteSession>().moving.set(Some(id));
        }),
        flat("add to-do", move || {
            commands::add_todo(todo_vm.clone(), todo_ui.clone(), id)
        }),
        flat("share", move || {
            share_vm.request(HostRequest::Share {
                title: share_note.title.clone(),
                text: share_note.body.clone(),
                url: String::new(),
            })
        }),
        flat("copy text", move || {
            copy_vm.request(HostRequest::CopyText {
                text: format!("{}\n\n{}", copy_note.title, copy_note.body),
            })
        }),
    ];
    page(
        "OneNote",
        body,
        app_bar(
            vec![
                action(Icon::Edit, "edit", move || {
                    commands::edit(edit_vm.clone(), edit_ui.clone(), editable.clone())
                }),
                action(Icon::Pin, "pin to start", move || {
                    pin.pin_note(id);
                    pin.home();
                }),
                action(Icon::Delete, "delete", move || {
                    let model = delete_vm.clone();
                    delete_ui.confirm(
                        "delete this note?",
                        Rc::new(move || {
                            model.delete_note(id);
                            model.navigate(Route::App(phone_domain::AppId::OneNote));
                        }),
                    );
                }),
                action(Icon::More, "more", move || {
                    more.menu.set(!more.menu.get_untracked())
                }),
            ],
            ui.menu.get(),
            menu,
        ),
    )
}
