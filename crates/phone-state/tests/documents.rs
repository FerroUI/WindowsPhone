use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use phone_domain::*;
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn document_mutations_are_atomic_and_preserve_folder_integrity() {
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.document_action(DocumentAction::CreateFolder {
        name: "Work".into(),
        parent: None,
    });
    let folder = vm.data().document_folders[0].id;
    vm.save_document(OfficeDocument::blank(
        DocumentKind::Word,
        "Letter".into(),
        Some(folder),
    ));
    let id = vm.data().documents[0].id;
    let next = vm.data().next_id;
    vm.save_document(OfficeDocument::blank(
        DocumentKind::Word,
        "letter".into(),
        Some(folder),
    ));
    assert_eq!(vm.data().documents.len(), 1);
    assert_eq!(vm.data().next_id, next);
    vm.document_action(DocumentAction::DeleteFolder { id: folder });
    assert_eq!(vm.data().document_folders.len(), 1);
    vm.document_action(DocumentAction::Copy {
        id,
        folder: Some(folder),
    });
    assert_eq!(vm.data().documents.len(), 2);
    assert_eq!(vm.data().documents[1].name, "Letter (2)");
    vm.document_action(DocumentAction::Move {
        ids: vec![id],
        folder: None,
    });
    assert_eq!(vm.data().document(id).unwrap().folder, None);
    let json = vm.data().to_json().unwrap();
    assert_eq!(PhoneData::from_json(&json).unwrap(), *vm.data());
}

#[test]
fn deleting_an_open_document_repairs_navigation() {
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.save_document(OfficeDocument::blank(
        DocumentKind::Excel,
        "Book".into(),
        None,
    ));
    let id = vm.data().documents[0].id;
    vm.navigate(Route::Document(id));
    assert_eq!(vm.route(), Route::Document(id));
    vm.document_action(DocumentAction::Delete(vec![id]));
    assert_ne!(vm.route(), Route::Document(id));
}

#[test]
fn document_shortcuts_stay_valid_through_rename_delete_and_backup() {
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    for (kind, name) in [
        (DocumentKind::Word, "Letter"),
        (DocumentKind::Excel, "Book"),
    ] {
        vm.save_document(OfficeDocument::blank(kind, name.into(), None));
    }
    let word = vm.data().documents[0].id;
    let book = vm.data().documents[1].id;
    vm.pin_document(word);
    vm.pin_document(book);
    let tile_for = |id| {
        vm.data()
            .all_tiles()
            .find(|tile| tile.target == Some(TileTarget::Document(id)))
            .unwrap()
            .id
    };
    let word_tile = tile_for(word);
    let book_tile = tile_for(book);
    vm.create_folder(word_tile, book_tile, "Documents".into());
    let folder = vm.data().parent_folder(word_tile).unwrap();
    vm.document_action(DocumentAction::Rename {
        id: word,
        name: "Renamed letter".into(),
    });
    assert_eq!(vm.data().tile(word_tile).unwrap().label, "Renamed letter");
    vm.open_tile(word_tile);
    assert_eq!(vm.route(), Route::Document(word));
    let backup = PhoneData::from_json(&vm.export_json().unwrap()).unwrap();
    assert_eq!(*vm.data(), backup);
    assert_eq!(backup.parent_folder(word_tile), Some(folder));
    vm.document_action(DocumentAction::Delete(vec![word]));
    assert!(vm.data().tile(word_tile).is_none());
    assert!(vm.data().tile(folder).is_none());
    assert!(vm.data().tile(book_tile).is_some());
    assert_eq!(vm.data().parent_folder(book_tile), None);
    assert_ne!(vm.route(), Route::Document(word));
    vm.document_action(DocumentAction::Delete(vec![book]));
    assert!(vm.data().tile(book_tile).is_none());
    assert_eq!(
        *vm.data(),
        PhoneData::from_json(&vm.export_json().unwrap()).unwrap()
    );
}

#[test]
fn import_returns_committed_identity_and_rejects_reentrant_insertion() {
    let saves = Rc::new(Cell::new(0));
    let saved = saves.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| saved.set(saved.get() + 1)),
    );
    let attempted = Rc::new(Cell::new(false));
    let attempted_once = attempted.clone();
    let result = Rc::new(RefCell::new(None));
    let output = result.clone();
    let weak = Rc::downgrade(&vm);
    let subscription = vm.changed().subscribe_next(move |_| {
        if !attempted_once.replace(true) {
            if let Some(vm) = weak.upgrade() {
                *output.borrow_mut() = Some(vm.import_document(OfficeDocument::blank(
                    DocumentKind::Word,
                    "Reentrant import".into(),
                    None,
                )));
            }
        }
    });
    let first_id = vm.data().next_id;
    vm.save_document(OfficeDocument::blank(
        DocumentKind::Word,
        "First".into(),
        None,
    ));
    subscription.dispose();
    assert!(result.borrow().as_ref().is_some_and(Result::is_err));
    assert_eq!(vm.data().documents.len(), 1);
    assert_eq!(vm.data().next_id, first_id + 1);
    assert_eq!(saves.get(), 1);

    vm.set_status("Earlier failed operation".into());
    let id = vm
        .import_document(OfficeDocument::blank(
            DocumentKind::Excel,
            "Imported".into(),
            None,
        ))
        .unwrap();
    assert_eq!(id, first_id + 1);
    assert_eq!(vm.data().document(id).unwrap().name, "Imported");
    assert_eq!(vm.data().next_id, id + 1);
    assert_eq!(vm.data().documents.len(), 2);
    assert_eq!(saves.get(), 2);
    assert!(vm.status().is_empty());
}

#[test]
fn file_batches_validate_all_ids_deduplicate_and_persist_once() {
    let saves = Rc::new(Cell::new(0));
    let saved = saves.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| saved.set(saved.get() + 1)),
    );
    vm.save_document(OfficeDocument::blank(
        DocumentKind::Word,
        "First".into(),
        None,
    ));
    vm.save_document(OfficeDocument::blank(
        DocumentKind::Word,
        "Second".into(),
        None,
    ));
    let a = vm.data().documents[0].id;
    let b = vm.data().documents[1].id;
    let before = vm.data().clone();
    let count = saves.get();
    for action in [
        DocumentAction::Delete(vec![a, u64::MAX]),
        DocumentAction::CopyMany {
            ids: vec![a, u64::MAX],
            folder: None,
        },
        DocumentAction::Move {
            ids: vec![a, u64::MAX],
            folder: None,
        },
        DocumentAction::Move {
            ids: vec![a],
            folder: Some(u64::MAX),
        },
    ] {
        vm.clear_status();
        vm.document_action(action);
        assert!(!vm.status().is_empty());
        assert_eq!(*vm.data(), before);
        assert_eq!(saves.get(), count);
    }
    vm.clear_status();
    vm.document_action(DocumentAction::CopyMany {
        ids: vec![a, a, b, b],
        folder: None,
    });
    assert_eq!(vm.data().documents.len(), 4);
    assert_eq!(vm.data().next_id, before.next_id + 2);
    assert_eq!(saves.get(), count + 1);
    vm.document_action(DocumentAction::Delete(vec![a, a, b, b]));
    assert_eq!(vm.data().documents.len(), 2);
    assert_eq!(saves.get(), count + 2);
}

#[test]
fn moving_to_the_same_folder_and_renaming_to_the_same_name_are_noops() {
    let mut data = PhoneData::default();
    let id = data.next_id;
    data.next_id += 1;
    data.documents.push(OfficeDocument {
        id,
        modified: 1,
        name: "Unchanged".into(),
        ..Default::default()
    });
    let original = data.clone();
    let saves = Rc::new(Cell::new(0));
    let saved = saves.clone();
    let vm = PhoneViewModel::new(data, Rc::new(move |_| saved.set(saved.get() + 1)));
    vm.document_action(DocumentAction::Move {
        ids: vec![id, id],
        folder: None,
    });
    vm.document_action(DocumentAction::Rename {
        id,
        name: " Unchanged ".into(),
    });
    assert_eq!(*vm.data(), original);
    assert_eq!(saves.get(), 0);
}
