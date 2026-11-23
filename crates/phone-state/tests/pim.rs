use ferroui_dynamic_data::IObservableCache;
use phone_domain::{interchange::*, *};
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn fixture() -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    (
        PhoneViewModel::new(
            PhoneData::default(),
            Rc::new(move |_| count.set(count.get() + 1)),
        ),
        writes,
    )
}
fn cards() -> Vec<u8> {
    export_vcards(&[
        Contact {
            name: "Alex".into(),
            email: "alex@example.test".into(),
            ..Default::default()
        },
        Contact {
            name: "Sam".into(),
            phone: "5550100".into(),
            ..Default::default()
        },
    ])
    .unwrap()
    .bytes
}

#[test]
fn contact_import_commits_one_snapshot_and_refreshes_projection_without_navigation_reset() {
    let (vm, writes) = fixture();
    vm.navigate(Route::App(AppId::People));
    let first = vm.data().next_id;
    let epoch = vm.document_epoch();
    let ids = vm.import_vcards(&cards()).unwrap();
    assert_eq!(ids, [first, first + 1]);
    assert_eq!(writes.get(), 1);
    assert_eq!(vm.contacts.items().len(), 2);
    assert_eq!(vm.data().next_id, first + 2);
    assert_eq!(vm.route(), Route::App(AppId::People));
    assert_eq!(vm.document_epoch(), epoch);
    assert_eq!(vm.schedule_revision(), 0);
    let snapshot = vm.data().clone();
    let output = vm.export_vcards(&[ids[0], ids[0], ids[1]]).unwrap();
    assert_eq!(import_vcards(&output.bytes).unwrap().len(), 2);
    assert_eq!(*vm.data(), snapshot);
    assert_eq!(writes.get(), 1);
}

#[test]
fn contact_import_failure_and_stale_export_leave_data_ids_and_writes_unchanged() {
    let (vm, writes) = fixture();
    let before = vm.data().clone();
    let mut invalid = cards();
    invalid.extend_from_slice(b"BEGIN:VCARD\nVERSION:4.0\nFN:Bad\nTEL:1\nTEL:2\nEND:VCARD\n");
    assert!(vm.import_vcards(&invalid).is_err());
    assert!(vm.export_vcards(&[999]).is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
    let mut exhausted = PhoneData::default();
    exhausted.next_id = Id::MAX - 2;
    let model = PhoneViewModel::new(exhausted, Rc::new(|_| panic!("must not persist")));
    assert!(model.import_vcards(&cards()).is_err());
    assert_eq!(model.data().next_id, Id::MAX - 2);
}

#[test]
fn calendar_import_invalidates_schedule_once_and_rejects_invalid_batch_atomically() {
    let (vm, writes) = fixture();
    let event = CalendarEvent {
        title: "Planning".into(),
        date: "2026-11-23".into(),
        time: "09:00".into(),
        duration_minutes: 60,
        repeat: CalendarRepeat::Daily,
        excluded_dates: vec!["2026-11-24".into()],
        reminder_minutes: Some(15),
        ..Default::default()
    };
    let file = export_icalendar(&[event], 1_795_420_000).unwrap();
    let ids = vm.import_icalendar(&file.bytes).unwrap();
    assert_eq!(writes.get(), 1);
    assert_eq!(vm.schedule_revision(), 1);
    assert_eq!(vm.data().events[0].id, ids[0]);
    let before = vm.data().clone();
    let invalid = String::from_utf8(file.bytes)
        .unwrap()
        .replace("FREQ=DAILY", "FREQ=DAILY;COUNT=10");
    assert!(vm.import_icalendar(invalid.as_bytes()).is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 1);
    assert_eq!(vm.schedule_revision(), 1);
    assert!(vm.export_icalendar(&[ids[0], 123]).is_err());
}

#[test]
fn imports_reject_reentrancy_and_drain_queued_commands_after_complete_publish() {
    let receiver = Rc::new(RefCell::new(std::rc::Weak::<PhoneViewModel>::new()));
    let weak = receiver.clone();
    let seen = Rc::new(Cell::new(0));
    let count = seen.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |data| {
            if count.get() == 0 {
                count.set(1);
                let vm = weak.borrow().upgrade().unwrap();
                assert_eq!(data.contacts.len(), 2);
                assert!(vm.import_vcards(&cards()).is_err());
                vm.dispatch(Action::ToggleFavoriteContact(data.contacts[0].id));
            } else {
                count.set(count.get() + 1);
            }
        }),
    );
    *receiver.borrow_mut() = Rc::downgrade(&vm);
    let ids = vm.import_vcards(&cards()).unwrap();
    assert!(vm.data().contact(ids[0]).unwrap().favorite);
    assert_eq!(vm.data().contacts.len(), 2);
    assert_eq!(seen.get(), 2);
}
