use super::*;
use phone_domain::{DocumentAction, validate_name};

pub(super) fn folder(
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    parent: Option<Id>,
    rename: Option<Id>,
) {
    let name = rename
        .and_then(|id| {
            vm.data()
                .document_folders
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.name.clone())
        })
        .unwrap_or_default();
    let error = ui.form_error;
    ui.form(
        if rename.is_some() {
            "rename folder"
        } else {
            "new folder"
        },
        vec![("name", name)],
        Rc::new(move |values| {
            let name = values
                .first()
                .map(String::as_str)
                .unwrap_or("")
                .trim()
                .to_owned();
            if let Err(message) = validate_name(&name) {
                error.set(message);
                return;
            }
            if vm.data().document_folders.iter().any(|f| {
                Some(f.id) != rename
                    && f.parent == parent
                    && f.name.to_lowercase() == name.to_lowercase()
            }) {
                error.set("This folder already exists.".into());
                return;
            }
            vm.clear_status();
            if let Some(id) = rename {
                vm.document_action(DocumentAction::RenameFolder { id, name });
            } else {
                vm.document_action(DocumentAction::CreateFolder { name, parent });
            }
            if !vm.status().is_empty() {
                error.set(vm.status());
            }
        }),
    );
}
pub(super) fn delete(vm: Rc<PhoneViewModel>, ui: Rc<Session>) {
    let ids = ui
        .files
        .selection
        .get_untracked()
        .into_iter()
        .collect::<Vec<_>>();
    if ids.is_empty() {
        vm.set_status("Select documents first.".into());
        return;
    }
    let state = ui.clone();
    ui.confirm(
        &format!(
            "delete {} document{}?",
            ids.len(),
            if ids.len() == 1 { "" } else { "s" }
        ),
        Rc::new(move || {
            vm.document_action(DocumentAction::Delete(ids.clone()));
            state.files.selection.set(BTreeSet::new());
            state.files.selecting.set(false);
        }),
    );
}
pub(super) fn transfer(vm: &PhoneViewModel, ui: &Session, copy: bool) {
    let ids = ui
        .files
        .selection
        .get_untracked()
        .into_iter()
        .collect::<Vec<_>>();
    if ids.is_empty() {
        vm.set_status("Select documents first.".into());
        return;
    }
    ui.files.transfer.set(Some(FileTransfer { ids, copy }));
    ui.files.selecting.set(false);
    ui.menu.set(false);
    ui.query.set(String::new());
    ui.files.searching.set(false);
    vm.navigate(Route::FileFolder(None));
}
pub(super) fn paste(vm: &PhoneViewModel, ui: &Session, folder: Option<Id>) {
    let Some(transfer) = ui.files.transfer.get_untracked() else {
        return;
    };
    let revision = vm.revision();
    vm.clear_status();
    if transfer.copy {
        vm.document_action(DocumentAction::CopyMany {
            ids: transfer.ids,
            folder,
        });
    } else {
        vm.document_action(DocumentAction::Move {
            ids: transfer.ids,
            folder,
        });
    }
    if vm.revision() != revision && vm.status().is_empty() {
        ui.files.transfer.set(None);
        ui.files.selection.set(BTreeSet::new());
    }
    // A no-op move into the same folder is still a completed transfer.
    if !transfer.copy && vm.status().is_empty() {
        ui.files.transfer.set(None);
        ui.files.selection.set(BTreeSet::new());
    }
}
pub(super) fn delete_folder(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id) {
    if vm
        .data()
        .document_folders
        .iter()
        .any(|f| f.parent == Some(id))
        || vm.data().documents.iter().any(|d| d.folder == Some(id))
    {
        vm.set_status("Move or delete this folder's contents first.".into());
        ui.menu.set(false);
        return;
    }
    ui.confirm(
        "delete this empty folder?",
        Rc::new(move || vm.document_action(DocumentAction::DeleteFolder { id })),
    );
}

pub(super) fn properties(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id) {
    let message = {
        let data = vm.data();
        let mut descendants = BTreeSet::from([id]);
        loop {
            let previous = descendants.len();
            for folder in &data.document_folders {
                if folder
                    .parent
                    .is_some_and(|parent| descendants.contains(&parent))
                {
                    descendants.insert(folder.id);
                }
            }
            if descendants.len() == previous {
                break;
            }
        }
        let documents = data
            .documents
            .iter()
            .filter(|document| {
                document
                    .folder
                    .is_some_and(|folder| descendants.contains(&folder))
            })
            .collect::<Vec<_>>();
        format!(
            "{} · {} documents, {} subfolders · {} of document text",
            breadcrumb(&vm, Some(id)),
            documents.len(),
            descendants.len().saturating_sub(1),
            size_text(documents.iter().map(|d| d.text_size()).sum())
        )
    };
    ui.menu.set(false);
    vm.set_status(message);
}
