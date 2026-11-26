use super::super::model::{Priority, Reminder};
use super::*;
use phone_domain::PhoneData;
use std::cell::Cell;

fn fixture() -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| count.set(count.get() + 1)),
    );
    vm.set_clock(1_795_665_600);
    (vm, writes)
}
fn fields(title: &str) -> Fields {
    Fields {
        title: title.into(),
        notes: "One line\n\n旅行 🙂".into(),
        due: Some("2026-11-26".into()),
        priority: Priority::High,
        reminder: Some(Reminder {
            date: "2026-11-25".into(),
            time: "08:15".into(),
        }),
    }
}

#[test]
fn full_task_lifecycle_round_trips_without_changing_calendar_events_or_other_namespaces() {
    let (vm, writes) = fixture();
    let epoch = vm.document_epoch();
    let events = vm.data().events.clone();
    AppStorage::new(vm.clone(), "test.other")
        .write(&serde_json::json!({"kept":true}))
        .unwrap();
    let first = save(&vm, epoch, None, fields("first")).unwrap();
    let second = save(&vm, epoch, None, fields("second")).unwrap();
    let before = writes.get();
    assert_eq!(
        save(&vm, epoch, Some(&first), first.fields.clone()).unwrap(),
        first
    );
    assert_eq!(writes.get(), before);
    let edited = save(&vm, epoch, Some(&first), fields("updated")).unwrap();
    assert_eq!(edited.id, first.id);
    assert_eq!(edited.revision, 2);
    let done = complete(&vm, epoch, &edited, true).unwrap();
    assert!(done.completed());
    let reopened = complete(&vm, epoch, &done, false).unwrap();
    assert!(!reopened.completed());
    assert_eq!(reopened.revision, 4);
    let restored = PhoneViewModel::new(
        serde_json::from_str(&vm.export_json().unwrap()).unwrap(),
        Rc::new(|_| {}),
    );
    assert_eq!(read(&restored).unwrap(), read(&vm).unwrap());
    delete(&restored, restored.document_epoch(), &[reopened]).unwrap();
    assert_eq!(read(&restored).unwrap().tasks, vec![second]);
    let later = save(&restored, restored.document_epoch(), None, fields("new")).unwrap();
    assert_eq!(later.id, 3);
    assert_eq!(restored.data().events, events);
    assert_eq!(
        restored.app_data("test.other"),
        Some(serde_json::json!({"kept":true}))
    );
}

#[test]
fn concurrent_edit_completion_bulk_delete_and_document_replacement_fail_without_partial_writes() {
    let (vm, writes) = fixture();
    let epoch = vm.document_epoch();
    let first = save(&vm, epoch, None, fields("first")).unwrap();
    let second = save(&vm, epoch, None, fields("second")).unwrap();
    let current = complete(&vm, epoch, &first, true).unwrap();
    let before = vm.export_json().unwrap();
    let count = writes.get();
    assert!(save(&vm, epoch, Some(&first), fields("stale")).is_err());
    assert!(complete(&vm, epoch, &first, false).is_err());
    assert!(delete(&vm, epoch, &[second.clone(), first.clone()]).is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    assert_eq!(writes.get(), count);
    delete(&vm, epoch, &[second, current]).unwrap();
    let previous = read(&vm).unwrap();
    vm.replace_data(PhoneData::default()).unwrap();
    let count = writes.get();
    assert!(save(&vm, epoch, None, fields("wrong document")).is_err());
    assert!(delete(&vm, epoch, &[first]).is_err());
    assert_eq!(writes.get(), count);
    assert!(read(&vm).unwrap().tasks.is_empty());
    assert_eq!(previous.next_id, 3);
}

#[test]
fn malformed_namespace_and_storage_budget_preserve_the_authoritative_document() {
    let (vm, writes) = fixture();
    let epoch = vm.document_epoch();
    vm.set_app_data(
        APP_ID,
        Some(serde_json::json!({"version":99,"next_id":1,"tasks":[]})),
    )
    .unwrap();
    let before = vm.export_json().unwrap();
    let count = writes.get();
    assert!(save(&vm, epoch, None, fields("must not erase unknown schema")).is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    assert_eq!(writes.get(), count);
    vm.set_app_data(APP_ID, None).unwrap();
    let mut large = fields("large");
    large.notes = "x".repeat(super::super::model::MAX_NOTES_BYTES);
    let mut rejected = false;
    for _ in 0..32 {
        let before = vm.export_json().unwrap();
        let count = writes.get();
        if save(&vm, epoch, None, large.clone()).is_err() {
            assert_eq!(vm.export_json().unwrap(), before);
            assert_eq!(writes.get(), count);
            rejected = true;
            break;
        }
    }
    assert!(
        rejected,
        "The namespace byte cap must reject the whole write before the task count cap"
    );
}
