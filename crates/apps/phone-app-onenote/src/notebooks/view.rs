use crate::{
    commands,
    notebooks::model::{Library, QUICK_NOTES},
    state::{NoteSession, Pane},
};
use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, flat, input, pivot, Icon};
use phone_domain::Note;
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, library: Library) -> Element {
    let pane = ui.state::<NoteSession>().pane.get();
    let recent = ui.clone();
    let notebooks = ui.clone();
    let mut body = stack_panel().spacing(8.0).child(rsx! {StackPanel{orientation:Orientation::Horizontal,margin:(20.0,0.0),
        {pivot("recent",pane == Pane::Recent,move || {recent.state::<NoteSession>().pane.set(Pane::Recent);recent.query.set(String::new());})}
        {pivot("notebooks",pane != Pane::Recent,move || {notebooks.state::<NoteSession>().pane.set(Pane::Notebooks);notebooks.query.set(String::new());})}
    }});
    let mut menu = Vec::new();
    let mut pages = None;
    match pane {
        Pane::Recent | Pane::Pages(_) => {
            let section = if let Pane::Pages(id) = pane {
                Some(id)
            } else {
                None
            };
            if let Some((notebook, section)) = section.and_then(|id| library.section(id)) {
                body = body.child(rsx! {StackPanel{margin:(20.0,4.0),spacing:4.0,
                {muted(&notebook.name,16.0)}{text(&section.name,30.0)}}});
                let id = section.id;
                let name = section.name.clone();
                let (model, state) = (vm.clone(), ui.clone());
                menu.push(flat("rename section", move || {
                    commands::rename(model.clone(), state.clone(), id, true, name.clone())
                }));
                if id != QUICK_NOTES {
                    let (model, state) = (vm.clone(), ui.clone());
                    menu.push(flat("delete section", move || {
                        commands::delete(model.clone(), state.clone(), id, true)
                    }));
                }
            }
            body = body.child(
                input("search notes")
                    .name("NoteSearch")
                    .text(ui.query.two_way())
                    .margin((20.0, 4.0)),
            );
            let query = ui.query.get().trim().to_lowercase();
            let mut notes = vm
                .data()
                .notes
                .iter()
                .filter(|note| {
                    section.is_none_or(|section| library.section_for(note.id) == section)
                        && (query.is_empty()
                            || note.title.to_lowercase().contains(&query)
                            || note.body.to_lowercase().contains(&query))
                })
                .cloned()
                .collect::<Vec<_>>();
            notes
                .sort_by_key(|note| (std::cmp::Reverse(note.modified), std::cmp::Reverse(note.id)));
            if notes.is_empty() {
                body = body.child(empty(
                    if query.is_empty() {
                        "Your ideas start here"
                    } else {
                        "No notes found"
                    },
                    "Tap new to write a note.",
                ));
            }
            let mut rows = Vec::with_capacity(notes.len());
            for note in notes {
                let preview = note
                    .body
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("Empty note")
                    .chars()
                    .take(100)
                    .collect::<String>();
                let progress = crate::checklist::summary(&note.body)
                    .map(|(done, total)| format!(" · {done}/{total} done"))
                    .unwrap_or_default();
                let location = library
                    .section(library.section_for(note.id))
                    .map(|(notebook, section)| format!("{} / {}", notebook.name, section.name))
                    .unwrap_or_default();
                let detail = if section.is_none() {
                    format!("{preview}\n{location}{progress}")
                } else {
                    format!("{preview}{progress}")
                };
                rows.push(super::pages::PageRow {
                    id: note.id,
                    title: note.title,
                    detail,
                });
            }
            if !rows.is_empty() {
                pages = Some(super::pages::view(rows, vm.clone()));
            }
        }
        Pane::Notebooks => {
            for notebook in &library.notebooks {
                let state = ui.clone();
                let id = notebook.id;
                body = body.child(row(
                    &notebook.name,
                    &format!("{} sections", notebook.sections.len()),
                    move || {
                        state.state::<NoteSession>().pane.set(Pane::Sections(id));
                        state.query.set(String::new());
                    },
                ));
            }
        }
        Pane::Sections(id) => {
            if let Some(notebook) = library.notebooks.iter().find(|notebook| notebook.id == id) {
                body = body.child(rsx! {Border{margin:(20.0,8.0),{text(&notebook.name,32.0)}}});
                if notebook.sections.is_empty() {
                    body = body.child(empty(
                        "No sections yet",
                        "Tap new section to organize your pages.",
                    ));
                }
                for section in &notebook.sections {
                    let state = ui.clone();
                    let id = section.id;
                    let count = vm
                        .data()
                        .notes
                        .iter()
                        .filter(|note| library.section_for(note.id) == id)
                        .count();
                    body = body.child(row(&section.name, &format!("{count} pages"), move || {
                        state.state::<NoteSession>().pane.set(Pane::Pages(id));
                        state.query.set(String::new());
                    }));
                }
                let name = notebook.name.clone();
                let (model, state) = (vm.clone(), ui.clone());
                menu.push(flat("rename notebook", move || {
                    commands::rename(model.clone(), state.clone(), id, false, name.clone())
                }));
                if !notebook
                    .sections
                    .iter()
                    .any(|section| section.id == QUICK_NOTES)
                {
                    let (model, state) = (vm.clone(), ui.clone());
                    menu.push(flat("delete notebook", move || {
                        commands::delete(model.clone(), state.clone(), id, false)
                    }));
                }
            }
        }
    }
    let (add_vm, add_ui) = (vm.clone(), ui.clone());
    let primary = match pane {
        Pane::Notebooks => action(Icon::Plus, "new notebook", move || {
            commands::new_notebook(add_vm.clone(), add_ui.clone())
        }),
        Pane::Sections(id) => action(Icon::Plus, "new section", move || {
            commands::new_section(add_vm.clone(), add_ui.clone(), id)
        }),
        _ => action(Icon::Plus, "new", move || {
            commands::edit(add_vm.clone(), add_ui.clone(), Note::default())
        }),
    };
    let (import_vm, import_ui) = (vm.clone(), ui.clone());
    let more = ui.clone();
    let contents = if let Some(pages) = pages {
        rsx! {Grid{row_definitions:"Auto,*",{body} Border{Grid.row:1,{pages}}}}
    } else {
        scroll(body.into())
    };
    page(
        "OneNote",
        contents,
        app_bar(
            vec![
                primary,
                action(Icon::Upload, "import note", move || {
                    crate::import::start(import_vm.clone(), import_ui.clone())
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

pub fn move_page(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    library: Library,
    id: phone_domain::Id,
) -> Element {
    let mut body = stack_panel()
        .spacing(8.0)
        .child(rsx! {Border{margin:20.0,{text("move page",36.0)}}});
    for notebook in &library.notebooks {
        body = body.child(rsx! {Border{margin:(20.0,12.0,20.0,0.0),{muted(&notebook.name,18.0)}}});
        for section in &notebook.sections {
            let (model, state) = (vm.clone(), ui.clone());
            let target = section.id;
            let current = library.section_for(id) == target;
            body = body.child(row(
                &section.name,
                if current { "current section" } else { "" },
                move || {
                    if model.data().note(id).is_none() {
                        model.set_status("This note no longer exists.".into());
                        return;
                    }
                    match super::change(&model, |library| library.move_page(id, target)) {
                        Ok(()) => state.state::<NoteSession>().moving.set(None),
                        Err(error) => model.set_status(error),
                    }
                },
            ));
        }
    }
    let cancel = ui.clone();
    page(
        "OneNote",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Close, "cancel", move || {
                cancel.state::<NoteSession>().moving.set(None)
            })],
            false,
            vec![],
        ),
    )
}
