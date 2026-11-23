use super::*;
use ferroui_base::layout::Orientation;
use phone_controls::{action, app_bar, flat, input, pivot};
use phone_domain::{AppId, DocumentKind};
use phone_state::HostRequest;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if ui
        .state::<phone_office_support::OfficeSession>()
        .show_new
        .get()
    {
        return new_page(vm, ui);
    }
    let recent = ui.pivot.get() == 0;
    let a = ui.clone();
    let b = ui.clone();
    let mut body=stack_panel().child(rsx!{StackPanel{orientation:Orientation::Horizontal,margin:(18.0,0.0),
        {pivot("recent",recent,move||a.pivot.set(0))}{pivot("places",!recent,move||b.pivot.set(1))}}});
    if ui
        .state::<phone_office_support::OfficeSession>()
        .show_find
        .get()
    {
        body = body.child(
            rsx! {Border{margin:(20.0,8.0),{input("search documents").text(ui.query.two_way())}}},
        );
    }
    if recent {
        let query = ui.query.get().to_lowercase();
        let mut documents = vm
            .data()
            .documents
            .iter()
            .filter(|d| d.name.to_lowercase().contains(&query))
            .cloned()
            .collect::<Vec<_>>();
        documents.sort_by_key(|d| std::cmp::Reverse(d.modified));
        if documents.is_empty() {
            body = body.child(empty(
                "take your work with you",
                "Tap new to create a document or import an existing Office file.",
            ));
        }
        for document in documents {
            let model = vm.clone();
            let context = ui.clone();
            let id = document.id;
            let detail = format!(
                "{} · phone · {}",
                document.kind().label(),
                date_text(document.modified)
            );
            body = body.child(rsx!{Border{
                on_context_requested:move|event|{event.set_handled(true);context.selected.set(Some(id));context.menu.set(true);},
                {document_row(&document, &detail, move || model.navigate(Route::Document(id)))}}});
        }
    } else {
        let phone = vm.clone();
        let onedrive = vm.clone();
        let sharepoint = vm.clone();
        body = body
            .child(folder_row(
                "phone",
                &format!("{} documents", vm.data().documents.len()),
                move || phone.navigate(Route::FileFolder(None)),
            ))
            .child(folder_row(
                "OneDrive",
                "open your files online",
                move || {
                    onedrive.request(HostRequest::OpenUrl {
                        url: "https://onedrive.live.com/".into(),
                    })
                },
            ))
            .child(folder_row(
                "Office 365",
                "open your work documents online",
                move || {
                    sharepoint.request(HostRequest::OpenUrl {
                        url: "https://www.microsoft365.com/".into(),
                    })
                },
            ));
        let notes = vm.clone();
        body = body.child(row("OneNote", "your notebooks", move || {
            notes.navigate(Route::App(AppId::OneNote))
        }));
    }
    let new = ui.clone();
    let search = ui.clone();
    let import = vm.clone();
    if ui.menu.get() {
        let document = ui
            .selected
            .get()
            .and_then(|id| vm.data().document(id).cloned());
        if let Some(document) = document {
            return page("office", scroll(body.into()), context_bar(vm, ui, document));
        }
    }
    page(
        "office",
        scroll(body.into()),
        app_bar(
            vec![
                action(Icon::Plus, "new", move || {
                    new.state::<phone_office_support::OfficeSession>()
                        .show_new
                        .set(true)
                }),
                action(Icon::Search, "search", move || {
                    search
                        .state::<phone_office_support::OfficeSession>()
                        .show_find
                        .set(
                            !search
                                .state::<phone_office_support::OfficeSession>()
                                .show_find
                                .get_untracked(),
                        )
                }),
                action(Icon::Upload, "import", move || {
                    import.request(HostRequest::ImportDocuments { folder: None })
                }),
            ],
            false,
            vec![],
        ),
    )
}

fn context_bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) -> Element {
    let close = ui.clone();
    let rename_vm = vm.clone();
    let rename_ui = ui.clone();
    let rename_doc = document.clone();
    let pin = vm.clone();
    let id = document.id;
    let export_vm = vm.clone();
    let export_ui = ui.clone();
    let export_doc = document.clone();
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    app_bar(
        vec![action(Icon::Close, "close", move || {
            close.menu.set(false);
            close.selected.set(None);
        })],
        true,
        vec![
            flat("pin to start", move || {
                pin.pin_document(id);
                pin.home();
            }),
            flat("rename", move || {
                phone_office_support::commands::rename(
                    rename_vm.clone(),
                    rename_ui.clone(),
                    rename_doc.clone(),
                )
            }),
            flat("save a copy", move || {
                phone_office_support::commands::export(
                    export_vm.clone(),
                    export_ui.clone(),
                    export_doc.clone(),
                )
            }),
            flat("delete", move || {
                phone_office_support::commands::delete(delete_vm.clone(), delete_ui.clone(), id)
            }),
        ],
    )
}
fn new_page(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let mut body = stack_panel().child(title("new"));
    for (kind, name, detail) in [
        (DocumentKind::Word, "Word document", "A blank document"),
        (DocumentKind::Excel, "Excel workbook", "A blank workbook"),
    ] {
        let model = vm.clone();
        let state = ui.clone();
        let document = OfficeDocument::blank(kind, name.into(), None);
        body = body.child(document_row(&document, detail, move || {
            phone_office_support::commands::create(model.clone(), state.clone(), kind, None)
        }));
    }
    let cancel = ui.clone();
    page(
        "office",
        scroll(body.into()),
        app_bar(
            vec![action(Icon::Back, "cancel", move || {
                cancel
                    .state::<phone_office_support::OfficeSession>()
                    .show_new
                    .set(false)
            })],
            false,
            vec![],
        ),
    )
}
