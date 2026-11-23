use crate::{
    notebooks::{self, model::QUICK_NOTES},
    state::{NoteSession, Pane},
};
use phone_domain::{Id, Note, Route};
use phone_sdk::session::Session;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, note: Note) {
    let state = ui.clone();
    let epoch = vm.document_epoch();
    let id = note.id;
    let section = match ui.state::<NoteSession>().pane.get_untracked() {
        Pane::Pages(section) => section,
        _ => QUICK_NOTES,
    };
    ui.form(
        if id == 0 { "new note" } else { "edit note" },
        vec![("title", note.title), ("note", note.body)],
        Rc::new(move |values| {
            let result = (|| {
                if vm.document_epoch() != epoch {
                    return Err("The phone data changed. Reopen the note before editing.".into());
                }
                let mut library = notebooks::read(&vm)?;
                let title = values
                    .first()
                    .map(String::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_owned();
                let body = values.get(1).cloned().unwrap_or_default();
                let expected = if id == 0 { vm.data().next_id } else { id };
                if id == 0 {
                    library.move_page(expected, section)?;
                }
                let value = serde_json::to_value(library).map_err(|error| error.to_string())?;
                vm.save_note_with_app_data(
                    Note {
                        id,
                        title,
                        body,
                        modified: vm.now(),
                    },
                    notebooks::APP_ID,
                    value,
                )
            })();
            match result {
                Ok(id) => {
                    state.menu.set(false);
                    vm.navigate(Route::Note(id));
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn new_notebook(vm: Rc<PhoneViewModel>, ui: Rc<Session>) {
    let state = ui.clone();
    ui.form(
        "new notebook",
        vec![("name", String::new())],
        Rc::new(move |values| {
            let result = notebooks::change(&vm, |library| library.add_notebook(name(&values)));
            match result {
                Ok(id) => {
                    state.state::<NoteSession>().pane.set(Pane::Sections(id));
                    state.menu.set(false);
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn new_section(vm: Rc<PhoneViewModel>, ui: Rc<Session>, notebook: u32) {
    let state = ui.clone();
    ui.form(
        "new section",
        vec![("name", String::new())],
        Rc::new(move |values| {
            let result =
                notebooks::change(&vm, |library| library.add_section(notebook, name(&values)));
            match result {
                Ok(id) => {
                    state.state::<NoteSession>().pane.set(Pane::Pages(id));
                    state.menu.set(false);
                }
                Err(error) => state.form_error.set(error),
            }
        }),
    );
}
pub fn rename(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: u32, section: bool, previous: String) {
    let state = ui.clone();
    ui.form(
        if section {
            "rename section"
        } else {
            "rename notebook"
        },
        vec![("name", previous)],
        Rc::new(move |values| {
            let result = notebooks::change(&vm, |library| {
                if section {
                    library.rename_section(id, name(&values))
                } else {
                    library.rename_notebook(id, name(&values))
                }
            });
            if let Err(error) = result {
                state.form_error.set(error);
            } else {
                state.menu.set(false);
            }
        }),
    );
}
pub fn delete(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: u32, section: bool) {
    let state = ui.clone();
    ui.confirm(
        if section {
            "delete section? Pages will move to Quick Notes."
        } else {
            "delete notebook? Pages will move to Quick Notes."
        },
        Rc::new(move || {
            let result = notebooks::change(&vm, |library| {
                if section {
                    library.delete_section(id)
                } else {
                    library.delete_notebook(id)
                }
            });
            match result {
                Ok(()) => {
                    state.state::<NoteSession>().pane.set(Pane::Notebooks);
                    state.menu.set(false);
                }
                Err(error) => vm.set_status(error),
            }
        }),
    );
}
pub fn add_todo(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id) {
    let state = ui.clone();
    let epoch = vm.document_epoch();
    ui.form_with_action(
        "add to-do",
        vec![("task", String::new())],
        "add",
        Rc::new(move |values| {
            let result = (|| {
                if vm.document_epoch() != epoch {
                    return Err("The phone data changed. Reopen the note.".into());
                }
                let label = name(&values);
                if label.is_empty() || label.contains(['\r', '\n']) {
                    return Err("Enter a task on one line.".into());
                }
                let mut note = vm
                    .data()
                    .note(id)
                    .cloned()
                    .ok_or("This note no longer exists.")?;
                if !note.body.is_empty() && !note.body.ends_with('\n') {
                    note.body.push('\n');
                }
                note.body.push_str("- [ ] ");
                note.body.push_str(&label);
                let library = notebooks::read(&vm)?;
                vm.save_note_with_app_data(
                    note,
                    notebooks::APP_ID,
                    serde_json::to_value(library).map_err(|e| e.to_string())?,
                )
            })();
            if let Err(error) = result {
                state.form_error.set(error);
            } else {
                state.menu.set(false);
            }
        }),
    );
}
fn name(values: &[String]) -> String {
    values
        .first()
        .map(String::as_str)
        .unwrap_or("")
        .trim()
        .to_owned()
}
