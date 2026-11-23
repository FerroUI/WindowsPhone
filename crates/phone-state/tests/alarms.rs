use phone_domain::*;
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

fn model(data: PhoneData) -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let count = writes.clone();
    (
        PhoneViewModel::new(data, Rc::new(move |_| count.set(count.get() + 1))),
        writes,
    )
}

#[test]
fn invalid_alarm_or_calendar_identifiers_and_capacity_are_atomic() {
    let mut data = PhoneData::default();
    data.alarms = (1..=MAX_ALARMS as u64)
        .map(|id| Alarm {
            id,
            hour: 7,
            label: "Alarm".into(),
            ..Default::default()
        })
        .collect();
    let (vm, writes) = model(data);
    let before = vm.data().clone();
    for id in [0, MAX_ALARMS as u64 + 50] {
        vm.save_alarm(Alarm {
            id,
            hour: 8,
            label: "Rejected alarm".into(),
            ..Default::default()
        });
        assert!(!vm.status().is_empty());
        assert_eq!(*vm.data(), before);
    }
    vm.save_event(CalendarEvent {
        id: 1000,
        title: "Missing appointment".into(),
        date: "2026-11-23".into(),
        time: "09:00".into(),
        duration_minutes: 60,
        ..Default::default()
    });
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
    assert_eq!(vm.schedule_revision(), 0);
}

#[test]
fn only_successful_schedule_mutations_invalidate_foreground_schedules() {
    let (vm, writes) = model(PhoneData::default());
    vm.set_clock(vm.now() + 120);
    vm.navigate(Route::App(AppId::Alarms));
    let mut settings = vm.data().settings.clone();
    settings.time_24h = true;
    vm.set_settings(settings);
    assert_eq!(vm.schedule_revision(), 0);
    vm.save_alarm(Alarm {
        hour: 7,
        label: "Wake up".into(),
        enabled: true,
        ..Default::default()
    });
    assert_eq!(vm.schedule_revision(), 1);
    let alarm = vm.data().alarms[0].clone();
    let before = writes.get();
    vm.save_alarm(alarm.clone());
    assert_eq!(writes.get(), before);
    assert_eq!(vm.schedule_revision(), 1);
    vm.toggle_alarm(alarm.id);
    assert_eq!(vm.schedule_revision(), 2);
    vm.save_alarm(Alarm {
        label: "x".repeat(257),
        ..alarm
    });
    assert_eq!(vm.schedule_revision(), 2);
    assert!(!vm.status().is_empty());
    vm.save_event(CalendarEvent {
        title: "Due now".into(),
        date: "2026-11-23".into(),
        time: "09:00".into(),
        duration_minutes: 30,
        reminder_minutes: Some(0),
        ..Default::default()
    });
    assert_eq!(vm.schedule_revision(), 3);
    let id = vm.data().events[0].id;
    vm.delete_event(id);
    assert_eq!(vm.schedule_revision(), 4);
}
