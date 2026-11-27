//! The two Office hub panes retain independent vertical scroll positions.
use super::*;
use ferroui_controls::primitives::ScrollBarVisibility;
use phone_domain::AppId;

pub(super) fn recent(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let query = ui.query.get().trim().to_lowercase();
    let mut documents = vm
        .data()
        .documents
        .iter()
        .filter(|document| document.name.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    documents.sort_by_key(|document| std::cmp::Reverse(document.modified));
    let mut body = stack_panel();
    if documents.is_empty() {
        body = body.child(if query.is_empty() {
            empty(
                "take your work with you",
                "Tap new to create a document or import an existing Office file.",
            )
        } else {
            empty("no documents found", "Try a different name.")
        });
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
        body = body.child(rsx! {Border {key:format!("office-document-{id}"),
            on_context_requested:move |event| {
                event.set_handled(true);
                context.selected.set(Some(id));
                context.menu.set(true);
            },
            {document_row(&document, &detail, move || model.navigate(Route::Document(id)))}
        }});
    }
    pane("OfficeRecentScroll", body.into())
}

pub(super) fn places(vm: Rc<PhoneViewModel>) -> Element {
    let phone = vm.clone();
    let onedrive = vm.clone();
    let sharepoint = vm.clone();
    let notes = vm.clone();
    pane(
        "OfficePlacesScroll",
        rsx! {StackPanel {
            {folder_row("phone", &format!("{} documents", vm.data().documents.len()),
                move || phone.navigate(Route::FileFolder(None)))}
            {folder_row("OneDrive", "open your files online", move || {
                onedrive.request(HostRequest::OpenUrl { url:"https://onedrive.live.com/".into() })
            })}
            {folder_row("Office 365", "open your work documents online", move || {
                sharepoint.request(HostRequest::OpenUrl { url:"https://www.microsoft365.com/".into() })
            })}
            {row("OneNote", "your notebooks", move || notes.navigate(Route::App(AppId::OneNote)))}
        }},
    )
}

fn pane(name: &str, content: Element) -> Element {
    rsx! {ScrollViewer { name:name,
        horizontal_scroll_bar_visibility:ScrollBarVisibility::Disabled,
        vertical_scroll_bar_visibility:ScrollBarVisibility::Hidden,
        {content}
    }}
}
