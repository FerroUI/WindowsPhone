use super::*;
use ferroui_view::prelude::{effect, Owner};
use phone_domain::{AppId, CalendarEvent, CalendarOccurrence, CalendarReminder, CalendarRepeat};

const CALENDAR: &str = "phone.calendar";

fn calendar_registry() -> Rc<AppRegistry> {
    Rc::new(
        AppRegistry::new(vec![AppRegistration::builtin(AppId::Calendar, |_, _| {
            Element::Empty
        })
        .with_activation(|_, _, _| Ok(()))])
        .unwrap(),
    )
}

fn fixture() -> Fixture {
    let mut data = PhoneData::default();
    data.alarms.clear();
    data.events = vec![CalendarEvent {
        id: 42,
        title: "Private recurring appointment".into(),
        location: "Private meeting room".into(),
        date: "2026-11-25".into(),
        time: "09:15".into(),
        duration_minutes: 90,
        repeat: CalendarRepeat::Daily,
        reminder_minutes: Some(15),
        ..Default::default()
    }];
    data.settings.notifications_on_lock = false;
    Fixture::with_data(data, calendar_registry())
}

fn fire(f: &Fixture) -> CalendarReminder {
    f.state.tick_with_registry(&f.vm, &f.registry);
    match f.state.active.get_untracked() {
        Some(Alert::Calendar(item)) => item,
        _ => panic!("the recurring appointment should be active"),
    }
}

struct Effects(Owner);
impl Drop for Effects {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

#[derive(Clone, Copy, Debug)]
enum Invalidation {
    Document,
    Session,
    Suspend,
    Close,
    Registry,
    Removed,
    Rescheduled,
}

const INVALIDATIONS: [Invalidation; 7] = [
    Invalidation::Document,
    Invalidation::Session,
    Invalidation::Suspend,
    Invalidation::Close,
    Invalidation::Registry,
    Invalidation::Removed,
    Invalidation::Rescheduled,
];

fn invalidate(
    f: &Fixture,
    action: Invalidation,
    retained_session: Rc<RefCell<Option<Rc<Session>>>>,
) -> Callback {
    let vm = f.vm.clone();
    let state = f.state.clone();
    let registry = f.registry.clone();
    let next_registry = calendar_registry();
    // A replacement shell owns its session independently of the observer that
    // publishes it. Creating it inside the unlock effect would instead make its
    // signals children of that effect, disposed by the next locked-state run.
    let replacement_ui = matches!(action, Invalidation::Session).then(|| {
        let ui = Rc::new(Session::new(vm.now(), registry));
        ui.locked.set(true);
        ui
    });
    Box::new(move || match action {
        Invalidation::Document => {
            let data = vm.data().clone();
            vm.replace_data(data).unwrap();
        }
        Invalidation::Session => {
            let ui = replacement_ui.expect("the independent replacement session is prepared");
            phone_sdk::set_current_session(&ui);
            *retained_session.borrow_mut() = Some(ui);
        }
        Invalidation::Suspend => state.host_visibility(&vm, false),
        Invalidation::Close => state.host_closing(&vm),
        Invalidation::Registry => state.tick_with_registry(&vm, &next_registry),
        Invalidation::Removed => vm.delete_event(42),
        Invalidation::Rescheduled => {
            let mut event = vm.data().events[0].clone();
            event.time = "10:15".into();
            vm.save_event(event);
        }
    })
}

#[test]
fn calendar_open_preserves_the_exact_occurrence_and_rejects_retained_callbacks() {
    let f = fixture();
    let item = fire(&f);
    let epoch = f.vm.document_epoch();
    assert_eq!(item.occurrence.date, "2026-11-26");
    assert_eq!(item.occurrence.end - item.occurrence.start, 90 * 60);
    f.state.open_calendar_reminder(&f.vm, &f.ui, epoch, &item);
    assert_eq!(f.vm.route(), Route::App(AppId::Calendar));
    assert!(!f.ui.locked.get_untracked());
    assert!(!f.vm.data().settings.notifications_on_lock);
    assert!(f.state.active.get_untracked().is_none());
    assert_eq!(
        f.ui.take_activation(CALENDAR),
        Some(serde_json::to_value(&item.occurrence).unwrap())
    );

    f.vm.navigate(Route::Start);
    f.ui.locked.set(true);
    f.state.open_calendar_reminder(&f.vm, &f.ui, epoch, &item);
    assert_eq!(f.vm.route(), Route::Start);
    assert!(f.ui.locked.get_untracked());
    assert!(f.ui.take_activation(CALENDAR).is_none());

    // Restore an equal event/occurrence ID and make its new alert active. The
    // old rendered callback must still fail on the original document epoch.
    let data = f.vm.data().clone();
    f.vm.replace_data(data).unwrap();
    f.state.clear();
    let replacement = fire(&f);
    assert_eq!(replacement, item);
    let before = f.requests.borrow().len();
    f.state.open_calendar_reminder(&f.vm, &f.ui, epoch, &item);
    assert_eq!(f.vm.route(), Route::Start);
    assert!(f.ui.locked.get_untracked());
    assert!(f.ui.take_activation(CALENDAR).is_none());
    assert_eq!(f.requests.borrow().len(), before);
    assert!(
        matches!(f.state.active.get_untracked(), Some(Alert::Calendar(active)) if active == replacement)
    );
}

#[test]
fn calendar_open_rechecks_the_owner_and_occurrence_after_synchronous_dismissal() {
    for action in INVALIDATIONS {
        let f = fixture();
        let item = fire(&f);
        let epoch = f.vm.document_epoch();
        let replacement = Rc::new(RefCell::new(None));
        let invalidation = invalidate(&f, action, replacement.clone());
        let ran = Rc::new(Cell::new(false));
        let recorded = ran.clone();
        *f.on_stop.borrow_mut() = Some(Box::new(move || {
            recorded.set(true);
            invalidation();
        }));
        f.state.open_calendar_reminder(&f.vm, &f.ui, epoch, &item);
        assert!(ran.get(), "stop-audio callback did not run: {action:?}");
        assert_eq!(f.vm.route(), Route::Start, "stale route: {action:?}");
        assert!(f.ui.locked.get_untracked(), "stale unlock: {action:?}");
        assert!(
            f.ui.take_activation(CALENDAR).is_none(),
            "stale payload: {action:?}"
        );
        let replacement_ui = replacement.borrow().clone();
        if let Some(ui) = replacement_ui {
            assert!(ui.locked.get_untracked());
            assert!(ui.take_activation(CALENDAR).is_none());
        }
    }
}

#[test]
fn calendar_open_rechecks_reentrant_unlock_before_publishing_activation() {
    for action in INVALIDATIONS {
        let f = fixture();
        let item = fire(&f);
        let epoch = f.vm.document_epoch();
        let replacement = Rc::new(RefCell::new(None));
        let invalidation = Rc::new(RefCell::new(Some(invalidate(
            &f,
            action,
            replacement.clone(),
        ))));
        let ran = Rc::new(Cell::new(false));
        let effects = Effects(Owner::new_root());
        effects.0.with(|| {
            let ui = f.ui.clone();
            let invalidation = invalidation.clone();
            let ran = ran.clone();
            effect(move || {
                if !ui.locked.get() {
                    let action = invalidation.borrow_mut().take();
                    if let Some(action) = action {
                        ran.set(true);
                        // A synchronous privacy takeover owns this newer lock
                        // state; the old open must not unlock or navigate again.
                        ui.locked.set(true);
                        action();
                    }
                }
            });
        });
        f.state.open_calendar_reminder(&f.vm, &f.ui, epoch, &item);
        assert!(ran.get(), "unlock callback did not run: {action:?}");
        assert_eq!(f.vm.route(), Route::Start, "stale route: {action:?}");
        assert!(
            f.ui.locked.get_untracked(),
            "privacy takeover lost: {action:?}"
        );
        assert!(
            f.ui.take_activation(CALENDAR).is_none(),
            "stale payload: {action:?}"
        );
        let replacement_ui = replacement.borrow().clone();
        if let Some(ui) = replacement_ui {
            assert!(ui.locked.get_untracked());
            assert!(ui.take_activation(CALENDAR).is_none());
        }
    }
}

#[test]
fn calendar_activation_does_not_overwrite_a_synchronous_consumer_navigation_or_restore() {
    for restore in [false, true] {
        let f = fixture();
        let item = fire(&f);
        let epoch = f.vm.document_epoch();
        f.vm.navigate(Route::App(AppId::Calendar));
        let received = Rc::new(RefCell::new(Vec::<CalendarOccurrence>::new()));
        let effects = Effects(Owner::new_root());
        effects.0.with(|| {
            let ui = f.ui.clone();
            let vm = f.vm.clone();
            let received = received.clone();
            effect(move || {
                if let Some(payload) = ui.take_activation(CALENDAR) {
                    received
                        .borrow_mut()
                        .push(serde_json::from_value(payload).unwrap());
                    if restore {
                        let data = vm.data().clone();
                        vm.replace_data(data).unwrap();
                    } else {
                        vm.navigate(Route::Setting("consumer navigation".into()));
                    }
                }
            });
        });
        f.state.open_calendar_reminder(&f.vm, &f.ui, epoch, &item);
        assert_eq!(*received.borrow(), [item.occurrence]);
        assert_eq!(
            f.vm.route(),
            if restore {
                Route::Start
            } else {
                Route::Setting("consumer navigation".into())
            }
        );
        assert_eq!(f.vm.document_epoch(), epoch + u64::from(restore));
        assert!(f.ui.take_activation(CALENDAR).is_none());
    }
}
