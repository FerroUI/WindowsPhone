use crate::{
    common::{empty, muted, page, row, scroll, text, title},
    session::Session,
};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use phone_controls::{Icon, action, app_bar};
use phone_domain::{Note, Route};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Route::Note(id) = vm.route() {
        let note = vm.data().note(id).cloned();
        if let Some(note) = note {
            return detail(vm, ui, note);
        }
    }
    let mut notes = vm.data().notes.clone();
    notes.sort_by_key(|note| std::cmp::Reverse(note.modified));
    let query = ui.query.get().to_lowercase();
    let list = notes
        .iter()
        .filter(|note| {
            format!("{} {}", note.title, note.body)
                .to_lowercase()
                .contains(&query)
        })
        .map(|note| {
            let model = vm.clone();
            let id = note.id;
            let preview = note
                .body
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("Empty note")
                .chars()
                .take(140)
                .collect::<String>();
            row(&note.title, &preview, move || {
                model.navigate(Route::Note(id))
            })
        })
        .collect::<Vec<_>>();
    let add_model = vm.clone();
    let add_ui = ui.clone();
    let body = rsx! { StackPanel {spacing:8.0,
        {title("notebooks")}
        TextBlock {text:"Personal",font_size:26.0,foreground:"#B077D0",margin:(20.0,0.0,20.0,8.0)}
        TextBox {class:"phone-input",text:ui.query.two_way(),placeholder_text:"search notes",automation_properties_name:"Search notes",margin:(20.0,0.0,20.0,8.0)}
        if list.is_empty() {{empty(if notes.is_empty(){"Your ideas start here"}else{"No notes found"},"Tap new to write a note.")}}
        for item in list {{item}}
    } };
    let bar = app_bar(
        vec![action(Icon::Plus, "new", move || {
            edit(add_model.clone(), add_ui.clone(), Note::default())
        })],
        false,
        Vec::new(),
    );
    page("OneNote", scroll(body), bar)
}

fn detail(vm: Rc<PhoneViewModel>, ui: Rc<Session>, note: Note) -> Element {
    let edit_model = vm.clone();
    let edit_ui = ui.clone();
    let editable = note.clone();
    let delete_model = vm.clone();
    let delete_ui = ui.clone();
    let pin = vm.clone();
    let id = note.id;
    let body = rsx! { StackPanel {margin:20.0,spacing:24.0,
        {text(&note.title,38.0)}
        TextBlock {text:note.body.clone(),font_size:22.0,text_wrapping:TextWrapping::Wrap,line_height:31.0}
        {muted("Saved on this device",14.0)}
    } };
    let bar = app_bar(
        vec![
            action(Icon::Edit, "edit", move || {
                edit(edit_model.clone(), edit_ui.clone(), editable.clone())
            }),
            action(Icon::Pin, "pin to start", move || {
                pin.pin_note(id);
                pin.home();
            }),
            action(Icon::Delete, "delete", move || {
                let model = delete_model.clone();
                delete_ui.confirm(
                    "delete this note?",
                    Rc::new(move || {
                        model.delete_note(id);
                        model.navigate(Route::App(phone_domain::AppId::OneNote));
                    }),
                );
            }),
        ],
        false,
        Vec::new(),
    );
    page("OneNote", scroll(body), bar)
}

fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, note: Note) {
    let error = ui.form_error;
    let id = note.id;
    ui.form(
        if id == 0 { "new note" } else { "edit note" },
        vec![("title", note.title), ("note", note.body)],
        Rc::new(move |values| {
            let title = values
                .first()
                .cloned()
                .unwrap_or_default()
                .trim()
                .to_owned();
            if title.is_empty() {
                error.set("Give your note a title.".into());
                return;
            }
            let body = values.get(1).cloned().unwrap_or_default();
            if body.len() > 1_048_576 {
                error.set("This note exceeds the 1 MB limit.".into());
                return;
            }
            vm.save_note(Note {
                id,
                title,
                body,
                modified: vm.now(),
            });
        }),
    );
}
