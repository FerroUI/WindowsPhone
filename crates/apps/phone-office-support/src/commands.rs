use super::*;
use phone_domain::{validate_document, validate_name, DocumentAction, DocumentKind, Id};
use phone_state::HostRequest;

pub fn create(vm: Rc<PhoneViewModel>, ui: Rc<Session>, kind: DocumentKind, folder: Option<Id>) {
    let error = ui.form_error;
    let state = ui.clone();
    let prefix = match kind {
        DocumentKind::Word => "Document",
        DocumentKind::Excel => "Book",
        DocumentKind::PowerPoint => "Presentation",
    };
    let mut count = 1;
    let mut name = format!("{prefix}{count}");
    while vm
        .data()
        .documents
        .iter()
        .any(|d| d.folder == folder && d.kind() == kind && d.name.eq_ignore_ascii_case(&name))
    {
        count += 1;
        name = format!("{prefix}{count}");
    }
    ui.form(
        &format!("new {}", kind.label().to_lowercase()),
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
            if vm.data().documents.iter().any(|d| {
                d.folder == folder && d.kind() == kind && d.name.eq_ignore_ascii_case(&name)
            }) {
                error.set("A document with this name already exists.".into());
                return;
            }
            let id = vm.data().next_id;
            vm.save_document(OfficeDocument::blank(kind, name, folder));
            if vm.data().document(id).is_some() {
                state.state::<crate::OfficeSession>().show_new.set(false);
                state.state::<crate::OfficeSession>().paragraph.set(0);
                state.state::<crate::OfficeSession>().sheet.set(0);
                state.state::<crate::OfficeSession>().slide.set(0);
                vm.navigate(Route::Document(id));
            } else {
                error.set(vm.status());
            }
        }),
    );
}
pub fn rename(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) {
    let error = ui.form_error;
    ui.form(
        "rename",
        vec![("name", document.name.clone())],
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
            if vm.data().documents.iter().any(|d| {
                d.id != document.id
                    && d.folder == document.folder
                    && d.kind() == document.kind()
                    && d.name.to_lowercase() == name.to_lowercase()
            }) {
                error.set("A document with this name already exists.".into());
                return;
            }
            vm.clear_status();
            vm.document_action(DocumentAction::Rename {
                id: document.id,
                name,
            });
            if !vm.status().is_empty() {
                error.set(vm.status());
            }
        }),
    );
}
pub fn delete(vm: Rc<PhoneViewModel>, ui: Rc<Session>, id: Id) {
    ui.confirm(
        "delete this document?",
        Rc::new(move || vm.document_action(DocumentAction::Delete(vec![id]))),
    );
}
pub fn export(vm: Rc<PhoneViewModel>, ui: Rc<Session>, document: OfficeDocument) {
    if document.import_notice.is_empty() {
        vm.request(HostRequest::ExportDocument { id: document.id });
    } else {
        ui.confirm(
            "export a mobile copy?",
            Rc::new(move || vm.request(HostRequest::ExportDocument { id: document.id })),
        );
    }
}
pub fn share(vm: &PhoneViewModel, document: &OfficeDocument) {
    vm.request(HostRequest::Share {
        title: document.name.clone(),
        text: document.plain_text(),
        url: String::new(),
    });
}
pub fn share_files(vm: Rc<PhoneViewModel>, ui: Rc<Session>, ids: Vec<Id>) {
    if ids.is_empty() {
        vm.set_status("Select files first.".into());
        return;
    }
    let eligibility = {
        let data = vm.data();
        if ids.iter().any(|id| data.document(*id).is_none()) {
            Err("A selected file no longer exists.")
        } else {
            Ok(ids.iter().any(|id| {
                data.document(*id)
                    .is_some_and(|d| !d.import_notice.is_empty())
            }))
        }
    };
    let imported = match eligibility {
        Ok(value) => value,
        Err(message) => {
            vm.set_status(message.into());
            return;
        }
    };
    ui.menu.set(false);
    if imported {
        ui.form_with_action(
            "share mobile copies?",
            vec![],
            "share",
            Rc::new(move |_| {
                vm.request(HostRequest::ShareDocuments { ids: ids.clone() });
            }),
        );
    } else {
        vm.request(HostRequest::ShareDocuments { ids });
    }
}
pub fn save(vm: &PhoneViewModel, ui: &Session, document: OfficeDocument) -> bool {
    if let Err(error) = validate_document(&document) {
        ui.form_error.set(error);
        return false;
    }
    let validation = {
        let data = vm.data();
        let mut documents = data.documents.clone();
        let Some(previous) = documents.iter_mut().find(|item| item.id == document.id) else {
            ui.form_error.set("This document no longer exists.".into());
            return false;
        };
        *previous = document.clone();
        phone_domain::validate_library(&documents, &data.document_folders)
    };
    if let Err(error) = validation {
        ui.form_error.set(error);
        return false;
    }
    vm.save_document(document);
    true
}
