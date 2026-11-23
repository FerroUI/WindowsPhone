use phone_domain::*;
use std::collections::BTreeSet;

impl crate::PhoneViewModel {
    /// Commit a freshly parsed document and return its durable identifier.
    /// A host expecting a synchronous result cannot enqueue an import inside
    /// another command: rejecting it leaves both the library and ID range intact.
    pub fn import_document(&self, document: OfficeDocument) -> Result<Id, String> {
        if self.applying.get() {
            return Err("A document import cannot run inside another command.".into());
        }
        if document.id != 0 {
            return Err("Imported documents must allocate a new identifier.".into());
        }
        self.clear_status();
        let id = self.data().next_id;
        self.dispatch(Action::Document(DocumentAction::Save(document)));
        if self.data().document(id).is_some() {
            Ok(id)
        } else {
            let error = self.status();
            Err(if error.is_empty() {
                "The document could not be imported.".into()
            } else {
                error
            })
        }
    }
}

/// Validate the candidate library before publishing any mutation or consuming IDs.
pub(crate) fn mutate(
    data: &mut PhoneData,
    action: DocumentAction,
    now: i64,
) -> Result<bool, String> {
    let mut documents = data.documents.clone();
    let mut folders = data.document_folders.clone();
    let mut next_id = data.next_id;
    let mut allocate = || {
        let id = next_id;
        next_id = next_id
            .checked_add(1)
            .ok_or("The document identifier range is exhausted.")?;
        Ok::<_, String>(id)
    };
    match action {
        DocumentAction::Save(mut document) => {
            document.name = document.name.trim().to_owned();
            validate_document(&document)?;
            if document.id == 0 {
                document.id = allocate()?;
                document.modified = now;
                documents.push(document);
            } else {
                let old = documents
                    .iter_mut()
                    .find(|d| d.id == document.id)
                    .ok_or("This document no longer exists.")?;
                if old.name == document.name
                    && old.folder == document.folder
                    && old.content == document.content
                    && old.import_notice == document.import_notice
                {
                    return Ok(false);
                }
                document.modified = now;
                *old = document;
            }
        }
        DocumentAction::Delete(ids) => {
            let selected = selection(&documents, &ids)?;
            documents.retain(|document| !selected.contains(&document.id));
        }
        DocumentAction::Rename { id, name } => {
            let document = documents
                .iter_mut()
                .find(|d| d.id == id)
                .ok_or("This document no longer exists.")?;
            let name = name.trim();
            if document.name == name {
                return Ok(false);
            }
            document.name = name.into();
            document.modified = now;
        }
        DocumentAction::Copy { id, folder } => {
            destination(&folders, folder)?;
            copy(&mut documents, &mut allocate, &[id], folder, now)?
        }
        DocumentAction::CopyMany { ids, folder } => {
            destination(&folders, folder)?;
            copy(&mut documents, &mut allocate, &ids, folder, now)?
        }
        DocumentAction::Move { ids, folder } => {
            destination(&folders, folder)?;
            let selected = selection(&documents, &ids)?;
            for document in &mut documents {
                if selected.contains(&document.id) && document.folder != folder {
                    document.folder = folder;
                    document.modified = now;
                }
            }
        }
        DocumentAction::CreateFolder { name, parent } => folders.push(DocumentFolder {
            id: allocate()?,
            name: name.trim().into(),
            parent,
        }),
        DocumentAction::RenameFolder { id, name } => {
            folders
                .iter_mut()
                .find(|f| f.id == id)
                .ok_or("This folder no longer exists.")?
                .name = name.trim().into()
        }
        DocumentAction::DeleteFolder { id } => {
            if folders.iter().any(|f| f.parent == Some(id))
                || documents.iter().any(|d| d.folder == Some(id))
            {
                return Err("Move or delete this folder's contents first.".into());
            }
            folders.retain(|folder| folder.id != id);
        }
    }
    validate_library(&documents, &folders)?;
    let changed = documents != data.documents || folders != data.document_folders;
    if changed {
        data.documents = documents;
        data.document_folders = folders;
        data.next_id = next_id;
    }
    Ok(changed)
}

fn copy(
    documents: &mut Vec<OfficeDocument>,
    allocate: &mut impl FnMut() -> Result<Id, String>,
    ids: &[Id],
    folder: Option<Id>,
    now: i64,
) -> Result<(), String> {
    let selected = selection(documents, ids)?;
    for id in selected {
        let mut document = documents
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or("A selected document no longer exists.")?;
        document.id = allocate()?;
        document.folder = folder;
        document.modified = now;
        let original = document.name.clone();
        let mut suffix = 1;
        while documents.iter().any(|d| {
            d.folder == folder
                && d.file_name().to_lowercase() == document.file_name().to_lowercase()
        }) {
            suffix += 1;
            document.name = format!(
                "{} ({suffix})",
                original.chars().take(108).collect::<String>()
            );
        }
        documents.push(document);
    }
    Ok(())
}

fn selection(documents: &[OfficeDocument], ids: &[Id]) -> Result<BTreeSet<Id>, String> {
    let selected = ids.iter().copied().collect::<BTreeSet<_>>();
    if selected
        .iter()
        .any(|id| !documents.iter().any(|d| d.id == *id))
    {
        return Err("A selected document no longer exists.".into());
    }
    Ok(selected)
}
fn destination(folders: &[DocumentFolder], folder: Option<Id>) -> Result<(), String> {
    if folder.is_some_and(|id| !folders.iter().any(|f| f.id == id)) {
        return Err("The destination folder no longer exists.".into());
    }
    Ok(())
}

impl crate::PhoneViewModel {
    pub fn save_document(&self, document: OfficeDocument) {
        self.dispatch(Action::Document(DocumentAction::Save(document)));
    }
    pub fn document_action(&self, action: DocumentAction) {
        self.dispatch(Action::Document(action));
    }
}
