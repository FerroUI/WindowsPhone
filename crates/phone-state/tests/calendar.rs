use phone_domain::*;
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

fn fixture() -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let mut data = PhoneData::default();
    data.events = vec![CalendarEvent {
        id: 7,
        title: "Daily planning".into(),
        date: "2026-11-23".into(),
        time: "09:00".into(),
        duration_minutes: 60,
        repeat: CalendarRepeat::Daily,
        ..Default::default()
    }];
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    (
        PhoneViewModel::new(data, Rc::new(move |_| count.set(count.get() + 1))),
        writes,
    )
}

#[test]
fn delete_one_occurrence_preserves_series_and_identity_without_allocating() {
    let (vm, writes) = fixture();
    let next_id = vm.data().next_id;
    vm.dispatch(Action::Calendar(CalendarAction::DeleteOccurrence {
        id: 7,
        date: "2026-11-24".into(),
    }));
    assert_eq!(writes.get(), 1);
    assert_eq!(vm.data().next_id, next_id);
    assert_eq!(vm.data().events.len(), 1);
    assert_eq!(vm.data().events[0].excluded_dates, ["2026-11-24"]);
    let before = vm.data().clone();
    vm.dispatch(Action::Calendar(CalendarAction::DeleteOccurrence {
        id: 7,
        date: "2026-11-24".into(),
    }));
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 1);
}

#[test]
fn editing_one_occurrence_commits_series_exception_and_standalone_replacement_once() {
    let (vm, writes) = fixture();
    let next_id = vm.data().next_id;
    let mut replacement = vm.data().events[0].clone();
    replacement.title = "Moved planning".into();
    replacement.date = "2026-11-25".into();
    replacement.time = "11:00".into();
    vm.dispatch(Action::Calendar(CalendarAction::SaveOccurrence {
        event: replacement,
        occurrence_date: "2026-11-24".into(),
    }));
    assert_eq!(writes.get(), 1);
    let data = vm.data();
    assert_eq!(data.events.len(), 2);
    assert_eq!(data.next_id, next_id + 1);
    assert_eq!(data.events[0].excluded_dates, ["2026-11-24"]);
    assert_eq!(data.events[1].id, next_id);
    assert_eq!(data.events[1].repeat, CalendarRepeat::Never);
    assert!(data.events[1].excluded_dates.is_empty());
    assert_eq!(data.events[1].date, "2026-11-25");
    assert_eq!(
        PhoneData::from_json(&data.to_json().unwrap()).unwrap(),
        *data
    );
}

#[test]
fn invalid_exception_edits_are_atomic_and_do_not_consume_identifiers() {
    let (vm, writes) = fixture();
    let before = vm.data().clone();
    let mut replacement = before.events[0].clone();
    replacement.title.clear();
    vm.dispatch(Action::Calendar(CalendarAction::SaveOccurrence {
        event: replacement,
        occurrence_date: "2026-11-24".into(),
    }));
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
    vm.dispatch(Action::Calendar(CalendarAction::DeleteOccurrence {
        id: 7,
        date: "2026-11-22".into(),
    }));
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
    vm.save_event(CalendarEvent {
        title: "invalid recurrence".into(),
        date: "2026-11-23".into(),
        time: "09:00".into(),
        duration_minutes: 60,
        repeat: CalendarRepeat::Daily,
        excluded_dates: vec!["2026-11-22".into()],
        ..Default::default()
    });
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
}
