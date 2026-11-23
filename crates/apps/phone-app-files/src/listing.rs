use super::*;
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::FontWeight,
};
use phone_controls::{flat, icon, input};
use phone_domain::{AppId, OfficeDocument};
use phone_office_support as office;
use phone_state::HostRequest;

pub(super) fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>, folder: Option<Id>) -> Element {
    let query = ui.query.get().to_lowercase();
    let mut folders = vm
        .data()
        .document_folders
        .iter()
        .filter(|f| f.parent == folder && f.name.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    folders.sort_by_key(|f| f.name.to_lowercase());
    let mut documents = vm
        .data()
        .documents
        .iter()
        .filter(|d| d.folder == folder && d.name.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    match ui.state::<crate::FilesSession>().sort.get() {
        1 => documents.sort_by_key(|d| std::cmp::Reverse(d.modified)),
        2 => documents.sort_by_key(|d| std::cmp::Reverse(d.text_size())),
        _ => documents.sort_by_key(|d| d.file_name().to_lowercase()),
    }
    let mut body = stack_panel()
        .spacing(2.0)
        .child(path_links(vm.clone(), ui.clone(), folder));
    if ui.state::<crate::FilesSession>().selecting.get() {
        body = body.child(rsx! {Border{margin:(20.0,4.0,20.0,8.0),
        {text(&format!("{} selected",ui.state::<crate::FilesSession>().selection.get().len()),24.0)}}});
    }
    if let Some(transfer) = ui.state::<crate::FilesSession>().transfer.get() {
        body=body.child(rsx!{Border{margin:(20.0,8.0),{muted(&format!("Choose where to {} {} document{}",if transfer.copy{"copy"}else{"move"},transfer.ids.len(),if transfer.ids.len()==1{""}else{"s"}),17.0)}}});
    }
    if ui.state::<crate::FilesSession>().searching.get() {
        body=body.child(rsx!{Border{margin:(20.0,4.0,20.0,12.0),{input("search this folder").text(ui.query.two_way())}}});
    }
    if folder.is_none()
        && query.is_empty()
        && !ui.state::<crate::FilesSession>().selecting.get()
        && ui.state::<crate::FilesSession>().transfer.get().is_none()
    {
        for (label, app, count) in [
            ("Pictures", AppId::Photos, vm.data().photos.len()),
            (
                "Music",
                AppId::Music,
                vm.data()
                    .tracks
                    .iter()
                    .filter(|t| t.media_kind != "video")
                    .count(),
            ),
            (
                "Videos",
                AppId::Video,
                vm.data()
                    .tracks
                    .iter()
                    .filter(|t| t.media_kind == "video")
                    .count(),
            ),
        ] {
            let model = vm.clone();
            body = body.child(office::folder_row(
                label,
                &format!("{count} items"),
                move || model.navigate(Route::App(app)),
            ));
        }
    }
    let empty_folder = folders.is_empty() && documents.is_empty();
    for item in folders {
        let count = vm
            .data()
            .documents
            .iter()
            .filter(|d| d.folder == Some(item.id))
            .count()
            + vm.data()
                .document_folders
                .iter()
                .filter(|f| f.parent == Some(item.id))
                .count();
        let model = vm.clone();
        let state = ui.clone();
        let context = ui.clone();
        let id = item.id;
        body=body.child(rsx!{Border{on_context_requested:move|event|{event.set_handled(true);context.state::<crate::FilesSession>().folder_menu.set(Some(id));context.menu.set(true);},
            {office::folder_row(&item.name,&format!("{count} items"),move||{state.query.set(String::new());state.state::<crate::FilesSession>().selection.set(BTreeSet::new());model.navigate(Route::FileFolder(Some(id)));})}}});
    }
    let visible = documents.iter().map(|d| d.id).collect::<BTreeSet<_>>();
    for document in documents {
        body = body.child(file_row(vm.clone(), ui.clone(), document));
    }
    if empty_folder {
        body = body.child(empty(
            if query.is_empty() {
                "no documents here"
            } else {
                "no results"
            },
            if query.is_empty() {
                "Import a document, or create a folder to organize your work."
            } else {
                "Try another name."
            },
        ));
    }
    let size = vm
        .data()
        .documents
        .iter()
        .filter(|d| d.folder == folder)
        .map(|d| d.text_size())
        .sum::<usize>();
    body=body.child(rsx!{Border{margin:(20.0,12.0),{muted(&format!("{} documents · {} of document text",visible.len(),size_text(size)),13.0)}}});
    let bar = bar(vm.clone(), ui.clone(), folder, visible);
    page("files", scroll(body.into()), bar)
}

fn file_row(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) -> Element {
    let id = document.id;
    let choosing = ui.state::<crate::FilesSession>().selecting.get();
    let selected = ui
        .state::<crate::FilesSession>()
        .selection
        .get()
        .contains(&id);
    let click = ui.clone();
    let context = ui.clone();
    let kind = document.kind();
    let open = vm.clone();
    rsx! {Button{class:"phone-flat",horizontal_alignment:H::Stretch,horizontal_content_alignment:H::Stretch,padding:(20.0,10.0),min_height:76.0,
        automation_properties_name:document.file_name(),automation_properties_help_text:format!("{} · {}",office::date_text(document.modified),size_text(document.text_size())),
        on_context_requested:move|event|{event.set_handled(true);context.state::<crate::FilesSession>().selecting.set(true);context.state::<crate::FilesSession>().selection.set(BTreeSet::from([id]));context.state::<crate::FilesSession>().folder_menu.set(None);context.menu.set(true);},
        on_click:move|_|{if click.state::<crate::FilesSession>().selecting.get_untracked(){let mut set=click.state::<crate::FilesSession>().selection.get_untracked();if !set.remove(&id){set.insert(id);}click.state::<crate::FilesSession>().selection.set(set);}else{open.navigate(Route::Document(id));}},
        Grid{column_definitions:if choosing{"32,44,*"}else{"0,44,*"},column_spacing:12.0,
            Border{is_visible:choosing,width:22.0,height:22.0,border_brush:fg(&vm).as_str(),border_thickness:2.0,vertical_alignment:V::Center,
                if selected{{icon(Icon::Check,16.0,&fg(&vm))}}}
            Border{Grid.column:1,width:42.0,height:50.0,background:kind.color(),
                TextBlock{text:kind.letter(),foreground:"White",font_size:29.0,font_weight:FontWeight::SemiBold,horizontal_alignment:H::Center,vertical_alignment:V::Center}}
            StackPanel{Grid.column:2,spacing:4.0,{text(&document.file_name(),23.0)}{muted(&format!("{} · {}",office::date_text(document.modified),size_text(document.text_size())),13.0)}}
    }}}
}

fn bar(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    folder: Option<Id>,
    visible: BTreeSet<Id>,
) -> Element {
    if let Some(transfer) = ui.state::<crate::FilesSession>().transfer.get() {
        let paste_vm = vm.clone();
        let paste_ui = ui.clone();
        let cancel = ui.clone();
        let add_vm = vm.clone();
        let add_ui = ui.clone();
        return app_bar(
            vec![
                action(
                    Icon::Check,
                    if transfer.copy {
                        "copy here"
                    } else {
                        "move here"
                    },
                    move || commands::paste(&paste_vm, &paste_ui, folder),
                ),
                action(Icon::Close, "cancel", move || {
                    cancel.state::<crate::FilesSession>().transfer.set(None)
                }),
                action(Icon::Folder, "new folder", move || {
                    commands::folder(add_vm.clone(), add_ui.clone(), folder, None)
                }),
            ],
            false,
            vec![],
        );
    }
    if ui.menu.get() {
        if let Some(id) = ui.state::<crate::FilesSession>().folder_menu.get() {
            let rename_vm = vm.clone();
            let rename_ui = ui.clone();
            let delete_vm = vm.clone();
            let delete_ui = ui.clone();
            let close = ui.clone();
            let properties_vm = vm.clone();
            let properties_ui = ui.clone();
            return app_bar(
                vec![action(Icon::Close, "close", move || {
                    close.menu.set(false);
                    close.state::<crate::FilesSession>().folder_menu.set(None);
                })],
                true,
                vec![
                    flat("rename folder", move || {
                        commands::folder(rename_vm.clone(), rename_ui.clone(), folder, Some(id))
                    }),
                    flat("delete empty folder", move || {
                        commands::delete_folder(delete_vm.clone(), delete_ui.clone(), id)
                    }),
                    flat("properties", move || {
                        commands::properties(properties_vm.clone(), properties_ui.clone(), id)
                    }),
                ],
            );
        }
    }
    if ui.state::<crate::FilesSession>().selecting.get() {
        return selection_bar(vm, ui, visible);
    }
    let search = ui.clone();
    let select = ui.clone();
    let add_vm = vm.clone();
    let add_ui = ui.clone();
    let more = ui.clone();
    let mut menu = Vec::new();
    for (index, label) in ["sort by name", "sort by date", "sort by size"]
        .into_iter()
        .enumerate()
    {
        let state = ui.clone();
        menu.push(flat(label, move || {
            state.state::<crate::FilesSession>().sort.set(index);
            state.menu.set(false);
        }));
    }
    let import = vm.clone();
    menu.push(flat("import documents", move || {
        import.request(HostRequest::ImportDocuments { folder })
    }));
    let create_vm = vm.clone();
    let create_ui = ui.clone();
    menu.push(flat("new Word document", move || {
        office::commands::create(
            create_vm.clone(),
            create_ui.clone(),
            phone_domain::DocumentKind::Word,
            folder,
        )
    }));
    app_bar(
        vec![
            action(Icon::Search, "search", move || {
                search.state::<crate::FilesSession>().searching.set(
                    !search
                        .state::<crate::FilesSession>()
                        .searching
                        .get_untracked(),
                );
                search.query.set(String::new());
            }),
            action(Icon::Check, "select", move || {
                select.state::<crate::FilesSession>().selecting.set(true);
                select
                    .state::<crate::FilesSession>()
                    .selection
                    .set(BTreeSet::new());
            }),
            action(Icon::Folder, "new folder", move || {
                commands::folder(add_vm.clone(), add_ui.clone(), folder, None)
            }),
            action(Icon::More, "more", move || {
                more.state::<crate::FilesSession>().folder_menu.set(None);
                more.menu.set(!more.menu.get_untracked());
            }),
        ],
        ui.menu.get(),
        menu,
    )
}
fn selection_bar(vm: Rc<PhoneViewModel>, ui: Rc<Session>, visible: BTreeSet<Id>) -> Element {
    let copy_vm = vm.clone();
    let copy_ui = ui.clone();
    let move_vm = vm.clone();
    let move_ui = ui.clone();
    let delete_vm = vm.clone();
    let delete_ui = ui.clone();
    let more = ui.clone();
    let mut menu = Vec::new();
    let selected = ui.state::<crate::FilesSession>().selection.get();
    let share_vm = vm.clone();
    let share_ui = ui.clone();
    menu.push(flat("share", move || {
        commands::share(share_vm.clone(), share_ui.clone())
    }));
    if selected.len() == 1 {
        let document = vm.data().document(*selected.first().unwrap()).cloned();
        if let Some(document) = document {
            let rename_vm = vm.clone();
            let rename_ui = ui.clone();
            let rename_doc = document.clone();
            menu.push(flat("rename", move || {
                office::commands::rename(rename_vm.clone(), rename_ui.clone(), rename_doc.clone())
            }));
            let export_vm = vm.clone();
            let export_ui = ui.clone();
            let export_doc = document.clone();
            menu.push(flat("export file", move || {
                office::commands::export(export_vm.clone(), export_ui.clone(), export_doc.clone())
            }));
            let share_vm = vm.clone();
            let property_vm = vm.clone();
            let property_ui = ui.clone();
            let id = document.id;
            menu.push(flat("properties", move || {
                commands::document_properties(property_vm.clone(), property_ui.clone(), id)
            }));
            menu.push(flat("share text", move || {
                office::commands::share(&share_vm, &document)
            }));
        }
    }
    let all = ui.clone();
    menu.push(flat("select all", move || {
        all.state::<crate::FilesSession>()
            .selection
            .set(visible.clone())
    }));
    let cancel = ui.clone();
    menu.push(flat("cancel selection", move || {
        cancel
            .state::<crate::FilesSession>()
            .selection
            .set(BTreeSet::new());
        cancel.state::<crate::FilesSession>().selecting.set(false);
        cancel.menu.set(false);
    }));
    app_bar(
        vec![
            action(Icon::Files, "copy", move || {
                commands::transfer(&copy_vm, &copy_ui, true)
            }),
            action(Icon::Folder, "move", move || {
                commands::transfer(&move_vm, &move_ui, false)
            }),
            action(Icon::Delete, "delete", move || {
                commands::delete(delete_vm.clone(), delete_ui.clone())
            }),
            action(Icon::More, "more", move || {
                more.menu.set(!more.menu.get_untracked())
            }),
        ],
        ui.menu.get(),
        menu,
    )
}
