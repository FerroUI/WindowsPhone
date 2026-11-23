//! Phone-local Files explorer. Its document operations share Office's persisted model.
mod commands;
mod listing;
mod properties;

use ferroui_view::prelude::*;
use phone_controls::{action, app_bar, Icon};
use phone_domain::{Id, Route};
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTransfer {
    pub ids: Vec<Id>,
    pub copy: bool,
}
pub struct FilesSession {
    pub selecting: Signal<bool>,
    pub selection: Signal<BTreeSet<Id>>,
    pub sort: Signal<usize>,
    pub searching: Signal<bool>,
    pub transfer: Signal<Option<FileTransfer>>,
    pub folder_menu: Signal<Option<Id>>,
    properties: Signal<Option<properties::FileProperties>>,
}
impl Default for FilesSession {
    fn default() -> Self {
        Self {
            selecting: signal(false),
            selection: signal(BTreeSet::new()),
            sort: signal(0),
            searching: signal(false),
            transfer: signal(None),
            folder_menu: signal(None),
            properties: signal(None),
        }
    }
}
impl FilesSession {
    pub fn dismiss_properties(&self) -> bool {
        let open = self.properties.get_untracked().is_some();
        if open {
            self.properties.set(None);
        }
        open
    }
    pub fn clear_properties(&self) {
        self.properties.set(None);
    }
}

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    if let Some(properties) = ui.state::<crate::FilesSession>().properties.get() {
        return properties::view(ui, properties);
    }
    if let Route::FileFolder(folder) = vm.route() {
        return listing::view(vm, ui, folder);
    }
    let size = vm
        .data()
        .documents
        .iter()
        .map(|d| d.text_size())
        .sum::<usize>();
    let open = vm.clone();
    let import = vm.clone();
    let backup = vm.clone();
    let body = rsx! {StackPanel{spacing:16.0,
        {title("files")}
        {phone_office_support::folder_row("phone",&format!("{} documents · {}",vm.data().documents.len(),size_text(size)),move||open.navigate(Route::FileFolder(None)))}
        Border{margin:(20.0,16.0),{muted("Browse the documents and media saved in this phone.",18.0)}}
    }};
    page(
        "files",
        scroll(body),
        app_bar(
            vec![
                action(Icon::Upload, "import", move || {
                    import.request(phone_state::HostRequest::ImportDocuments { folder: None })
                }),
                action(Icon::Save, "backup", move || {
                    backup.request(phone_state::HostRequest::ExportBackup)
                }),
            ],
            false,
            vec![],
        ),
    )
}
pub(crate) fn size_text(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
pub(crate) fn breadcrumb(vm: &PhoneViewModel, folder: Option<Id>) -> String {
    let data = vm.data();
    let mut names = Vec::new();
    let mut current = folder;
    for _ in 0..256 {
        let Some(id) = current else {
            break;
        };
        let Some(folder) = data.document_folders.iter().find(|f| f.id == id) else {
            break;
        };
        names.push(folder.name.clone());
        current = folder.parent;
    }
    names.push("phone".into());
    names.reverse();
    names.join(" › ")
}

pub(crate) fn path_links(vm: Rc<PhoneViewModel>, ui: Rc<Session>, folder: Option<Id>) -> Element {
    use ferroui_base::layout::Orientation;
    use ferroui_controls::primitives::ScrollBarVisibility;
    let mut links = Vec::new();
    let mut current = folder;
    {
        let data = vm.data();
        for _ in 0..256 {
            let Some(id) = current else {
                break;
            };
            let Some(item) = data.document_folders.iter().find(|f| f.id == id) else {
                break;
            };
            links.push((Some(id), item.name.clone()));
            current = item.parent;
        }
    }
    links.push((None, "phone".into()));
    links.reverse();
    let count = links.len();
    let mut row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .margin((16.0, 0.0, 12.0, 8.0));
    for (index, (destination, name)) in links.into_iter().enumerate() {
        let model = vm.clone();
        let state = ui.clone();
        row = row.child(phone_controls::flat(&name, move || {
            state.query.set(String::new());
            state
                .state::<crate::FilesSession>()
                .selection
                .set(BTreeSet::new());
            model.navigate(Route::FileFolder(destination));
        }));
        if index + 1 < count {
            row=row.child(rsx!{TextBlock{text:"›",font_size:24.0,foreground:"#777777",vertical_alignment:ferroui_base::layout::VerticalAlignment::Center}});
        }
    }
    rsx! {ScrollViewer{horizontal_scroll_bar_visibility:ScrollBarVisibility::Auto,vertical_scroll_bar_visibility:ScrollBarVisibility::Disabled,{row}}}
}

mod registration;
pub use registration::registration;
