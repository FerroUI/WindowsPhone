use super::*;
use phone_controls::{action, app_bar, flat, input};
use phone_domain::{DocumentContent, Id, Paragraph};
mod paragraphs;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) -> Element {
    let DocumentContent::Word(word) = &document.content else {
        return Element::Empty;
    };
    let selected = ui
        .office
        .paragraph
        .get()
        .min(word.paragraphs.len().saturating_sub(1));
    let query = ui.query.get().to_lowercase();
    let paper = paragraphs::view(word.paragraphs.clone(), query, ui.clone());
    let body = rsx! {Grid{row_definitions:"Auto,*,Auto",
        StackPanel{spacing:4.0,
            Border{margin:(20.0,4.0,20.0,8.0),{text(&document.name,28.0)}}
            if ui.office.show_find.get(){Border{margin:(20.0,4.0),{input("find in document").text(ui.query.two_way())}}}
            {compatibility(&document)}}
        Border{Grid.row:1,background:"White",margin:(12.0,4.0,12.0,12.0),{paper}}
        Border{Grid.row:2,margin:(20.0,0.0,20.0,12.0),{muted(&format!("{} words · paragraph {} of {}",word.paragraphs.iter().map(|p|p.text.split_whitespace().count()).sum::<usize>(),selected+1,word.paragraphs.len()),13.0)}}
    }};
    let id = document.id;
    let edit_vm = vm.clone();
    let edit_ui = ui.clone();
    let find = ui.clone();
    let more = ui.clone();
    let export_vm = vm.clone();
    let export_ui = ui.clone();
    let export_doc = document.clone();
    let mut menu = Vec::new();
    let operations = if ui.office.formatting.get() {
        vec![
            ("bold", 1),
            ("italic", 2),
            ("underline", 3),
            ("heading", 4),
            ("bullets", 5),
        ]
    } else {
        vec![("add paragraph", 0), ("delete paragraph", 6)]
    };
    if !ui.office.formatting.get() {
        let format = ui.clone();
        menu.push(flat("format", move || format.office.formatting.set(true)));
    }
    for (label, operation) in operations {
        let model = vm.clone();
        let state = ui.clone();
        menu.push(flat(label, move || {
            transform(&model, &state, id, operation)
        }));
    }
    if !ui.office.formatting.get() {
        let rename_vm = vm.clone();
        let rename_ui = ui.clone();
        let rename_doc = document.clone();
        menu.push(flat("rename", move || {
            super::commands::rename(rename_vm.clone(), rename_ui.clone(), rename_doc.clone())
        }));
        let pin = vm.clone();
        menu.push(flat("pin to start", move || {
            pin.pin_document(id);
            pin.home();
        }));
        let share_vm = vm.clone();
        let share_doc = document.clone();
        menu.push(flat("share text", move || {
            super::commands::share(&share_vm, &share_doc)
        }));
    }
    page(
        "word",
        body,
        app_bar(
            vec![
                action(Icon::Edit, "edit", move || {
                    edit(edit_vm.clone(), edit_ui.clone(), id)
                }),
                action(Icon::Search, "find", move || {
                    find.office
                        .show_find
                        .set(!find.office.show_find.get_untracked())
                }),
                action(Icon::Save, "save a copy", move || {
                    super::commands::export(
                        export_vm.clone(),
                        export_ui.clone(),
                        export_doc.clone(),
                    )
                }),
                action(Icon::More, "more", move || {
                    more.office.formatting.set(false);
                    more.menu.set(!more.menu.get_untracked());
                }),
            ],
            ui.menu.get(),
            menu,
        ),
    )
}

fn edit(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id) {
    let document = vm.data().document(id).cloned();
    let Some(document) = document else {
        return;
    };
    let DocumentContent::Word(word) = document.content else {
        return;
    };
    let index = ui
        .office
        .paragraph
        .get_untracked()
        .min(word.paragraphs.len() - 1);
    let state = ui.clone();
    ui.form(
        "edit document",
        vec![("body", word.paragraphs[index].text.clone())],
        Rc::new(move |values| {
            let document = vm.data().document(id).cloned();
            let Some(mut document) = document else {
                state
                    .form_error
                    .set("This document no longer exists.".into());
                return;
            };
            if let DocumentContent::Word(word) = &mut document.content {
                if let Some(paragraph) = word.paragraphs.get_mut(index) {
                    paragraph.text = values.first().cloned().unwrap_or_default();
                    super::commands::save(&vm, &state, document);
                }
            }
        }),
    );
}
fn transform(vm: &PhoneViewModel, ui: &Session, id: Id, operation: usize) {
    let document = vm.data().document(id).cloned();
    let Some(mut document) = document else {
        return;
    };
    if let DocumentContent::Word(word) = &mut document.content {
        let index = ui
            .office
            .paragraph
            .get_untracked()
            .min(word.paragraphs.len() - 1);
        match operation {
            0 => {
                word.paragraphs.insert(index + 1, Paragraph::default());
                ui.office.paragraph.set(index + 1);
            }
            1 => word.paragraphs[index].bold = !word.paragraphs[index].bold,
            2 => word.paragraphs[index].italic = !word.paragraphs[index].italic,
            3 => word.paragraphs[index].underline = !word.paragraphs[index].underline,
            4 => word.paragraphs[index].heading = (word.paragraphs[index].heading + 1) % 4,
            5 => word.paragraphs[index].bullet = !word.paragraphs[index].bullet,
            6 => {
                if word.paragraphs.len() > 1 {
                    word.paragraphs.remove(index);
                    ui.office.paragraph.set(index.saturating_sub(1));
                } else {
                    word.paragraphs[0] = Paragraph::default();
                }
            }
            _ => {}
        }
        vm.save_document(document);
        ui.menu.set(false);
    }
}
