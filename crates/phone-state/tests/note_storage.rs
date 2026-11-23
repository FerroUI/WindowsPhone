use phone_domain::{Note, PhoneData};
use phone_state::PhoneViewModel;
use serde_json::json;
use std::{cell::RefCell, rc::Rc};
#[test]
fn note_and_app_index_are_committed_once_together() {
    let writes = Rc::new(RefCell::new(Vec::new()));
    let saved = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |data| saved.borrow_mut().push(data.clone())),
    );
    let expected = vm.data().next_id;
    let index = json!({"section": 7, "note": expected.to_string()});
    let id = vm
        .save_note_with_app_data(
            Note {
                title: "Ideas".into(),
                body: "Check this".into(),
                ..Default::default()
            },
            "phone.onenote",
            index.clone(),
        )
        .unwrap();
    assert_eq!(id, expected);
    assert_eq!(writes.borrow().len(), 1);
    let snapshots = writes.borrow();
    assert_eq!(snapshots[0].note(id).unwrap().title, "Ideas");
    assert_eq!(snapshots[0].app_data["phone.onenote"], index);
}
#[test]
fn rejected_note_metadata_transaction_preserves_identifiers_and_data() {
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(|_| panic!("Rejected mutation must not save")),
    );
    let before = vm.export_json().unwrap();
    let note = Note {
        title: "Valid".into(),
        ..Default::default()
    };
    assert!(vm
        .save_note_with_app_data(note.clone(), "invalid/key", json!({}))
        .is_err());
    assert!(vm
        .save_note_with_app_data(note.clone(), "phone.onenote", json!({"bad":u64::MAX}))
        .is_err());
    assert!(vm
        .save_note_with_app_data(
            Note {
                body: "x".repeat(1_048_577),
                ..note.clone()
            },
            "phone.onenote",
            json!({})
        )
        .is_err());
    assert!(vm
        .save_note_with_app_data(Note { id: 8, ..note }, "phone.onenote", json!({}))
        .is_err());
    assert_eq!(vm.export_json().unwrap(), before);
}
