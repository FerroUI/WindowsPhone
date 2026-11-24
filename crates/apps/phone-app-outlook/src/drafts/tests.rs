use super::*;
use phone_domain::PhoneData;
use std::cell::Cell;

fn fixture() -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    (
        PhoneViewModel::new(
            PhoneData::demo(),
            Rc::new(move |_| count.set(count.get() + 1)),
        ),
        writes,
    )
}

fn fields(body: &str) -> Fields {
    Fields {
        to: "ada@example.com".into(),
        subject: "Project notes".into(),
        body: body.into(),
    }
}

#[test]
fn drafts_round_trip_multiline_content_update_in_place_and_delete_independently() {
    let (vm, writes) = fixture();
    let mail = vm.data().mail.clone();
    AppStorage::new(vm.clone(), "tests.other")
        .write(&serde_json::json!({"kept": true}))
        .unwrap();
    let first = save(&vm, None, fields("Line one\n\nLine three — Ελληνικά 🙂")).unwrap();
    let second = save(&vm, None, fields("Second draft")).unwrap();
    let before = writes.get();
    assert_eq!(
        save(&vm, Some(first.id), first.fields.clone()).unwrap(),
        first
    );
    assert_eq!(writes.get(), before, "An equal save does not persist again");
    let edited = save(&vm, Some(first.id), fields("Edited\ntext")).unwrap();
    assert_eq!(edited.id, first.id);
    assert_eq!(read(&vm).unwrap().items.len(), 2);

    let restored = PhoneViewModel::new(
        serde_json::from_str(&vm.export_json().unwrap()).unwrap(),
        Rc::new(|_| {}),
    );
    assert_eq!(read(&restored).unwrap(), read(&vm).unwrap());
    delete(&restored, first.id).unwrap();
    assert_eq!(read(&restored).unwrap().items, vec![second]);
    assert_eq!(
        restored.app_data("tests.other"),
        Some(serde_json::json!({"kept": true}))
    );
    assert_eq!(
        restored.data().mail,
        mail,
        "Draft operations never fabricate inbox or sent messages"
    );
}

#[test]
fn partial_recipients_can_be_saved_but_empty_or_invalid_headers_cannot() {
    let (vm, _) = fixture();
    let partial = Fields {
        to: "ada".into(),
        subject: String::new(),
        body: String::new(),
    };
    assert!(save(&vm, None, partial.clone()).is_ok());
    assert!(partial.email_request().is_err());
    assert!(save(&vm, None, Fields::default()).is_err());
    for invalid in [
        "ada@example.com\r\nBcc: someone@example.com",
        "ada@example.com\0",
    ] {
        let bad = Fields {
            to: invalid.into(),
            ..fields("text")
        };
        assert!(save(&vm, None, bad.clone()).is_err());
        assert!(bad.email_request().is_err());
    }
    let mut valid = fields("Line one\nLine two");
    valid.to = " ada@example.com, grace@example.com ".into();
    assert!(
        matches!(valid.email_request().unwrap(), HostRequest::Email { to, body, .. }
        if to == "ada@example.com, grace@example.com" && body == "Line one\nLine two")
    );
}

#[test]
fn rejected_budget_or_stale_edit_preserves_every_existing_draft() {
    let (vm, writes) = fixture();
    for _ in 0..3 {
        save(&vm, None, fields(&"x".repeat(MAX_BODY_BYTES))).unwrap();
    }
    let before = vm.data().clone();
    let count = writes.get();
    assert!(save(&vm, None, fields(&"x".repeat(MAX_BODY_BYTES))).is_err());
    assert!(save(&vm, Some(99), fields("stale")).is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), count);
}

#[test]
fn unknown_or_corrupt_schema_is_reported_without_replacing_stored_data() {
    let (vm, writes) = fixture();
    for value in [
        serde_json::json!({"version": 2, "next_id": 1, "items": []}),
        serde_json::json!({"version": 1, "next_id": 0, "items": []}),
        serde_json::json!({"version": 1, "next_id": 1, "items": [{"id": 1}]}),
    ] {
        AppStorage::new(vm.clone(), APP_ID).write(&value).unwrap();
        let count = writes.get();
        assert!(read(&vm).is_err());
        assert!(save(&vm, None, fields("retained editor text")).is_err());
        assert!(delete(&vm, 1).is_err());
        assert_eq!(vm.app_data(APP_ID), Some(value));
        assert_eq!(writes.get(), count);
    }
}
