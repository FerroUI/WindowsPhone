use super::*;
use ferroui_view::prelude::Element;
use phone_domain::{PhoneData, Route};
use phone_sdk::{AppManifest, AppRegistration, AppReminder, Capability, ScopedAppReminder};
use phone_state::HostServices;
use std::cell::RefCell;

const APP: &str = "test.reminders";
type Callback = Box<dyn FnOnce()>;
thread_local! {
    static COLLECT: RefCell<Option<Callback>> = const { RefCell::new(None) };
}

fn reminder(vm: &PhoneViewModel, title: &str) -> AppReminder {
    AppReminder {
        key: "task-1".into(),
        revision: 1,
        scheduled_local: vm.now().div_euclid(60) * 60,
        title: title.into(),
        detail: "A local reminder".into(),
        path: "task/1".into(),
    }
}

fn original(vm: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
    let callback = COLLECT.with(|slot| slot.borrow_mut().take());
    if let Some(callback) = callback {
        callback();
    }
    Ok(vec![reminder(vm, "Original")])
}

fn replacement(vm: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
    Ok(vec![reminder(vm, "Replacement")])
}

fn registry(provider: phone_sdk::ReminderProvider) -> Rc<AppRegistry> {
    Rc::new(
        AppRegistry::new(vec![AppRegistration::new(
            AppManifest::new(APP, "Reminders", phone_controls::Icon::Calendar)
                .capabilities(&[Capability::Notifications]),
            |_| Element::Empty,
        )
        .with_reminders(provider)])
        .unwrap(),
    )
}

struct Fixture {
    vm: Rc<PhoneViewModel>,
    ui: Rc<Session>,
    registry: Rc<AppRegistry>,
    state: Rc<AlarmState>,
    on_stop: Rc<RefCell<Option<Callback>>>,
    requests: Rc<RefCell<Vec<HostRequest>>>,
}

impl Fixture {
    fn new() -> Self {
        let mut data = PhoneData::default();
        data.alarms.clear();
        data.events.clear();
        data.settings.sounds = true;
        data.settings.ringer_volume = 7;
        let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
        vm.set_clock(phone_domain::days_from_date(2026, 11, 26) * 86_400 + 9 * 3_600);
        vm.set_utc_offset(0);
        let registry = registry(original);
        registry.bind(&vm);
        let ui = Rc::new(Session::new(vm.now(), registry.clone()));
        phone_sdk::set_current_session(&ui);
        let state = ui.state::<AlarmState>();
        ui.locked.set(true);
        let on_stop = Rc::new(RefCell::new(None::<Callback>));
        let requests = Rc::new(RefCell::new(Vec::new()));
        let callback = on_stop.clone();
        let recorded = requests.clone();
        vm.set_services(HostServices {
            request: Rc::new(move |request| {
                let stopped = matches!(&request, HostRequest::AlarmAlert { active: false, .. });
                recorded.borrow_mut().push(request);
                if stopped {
                    let action = callback.borrow_mut().take();
                    if let Some(action) = action {
                        action();
                    }
                }
            }),
        });
        Self {
            vm,
            ui,
            registry,
            state,
            on_stop,
            requests,
        }
    }

    fn fire(&self) -> ScopedAppReminder {
        self.state.tick_with_registry(&self.vm, &self.registry);
        match self.state.active.get_untracked() {
            Some(Alert::App(item)) => item,
            _ => panic!("the current task should be active"),
        }
    }
}

#[test]
fn opening_a_current_reminder_dismisses_and_navigates_once() {
    let f = Fixture::new();
    let item = f.fire();
    f.state.open_app_reminder(&f.vm, &f.ui, &item);
    assert!(!f.ui.locked.get_untracked());
    assert!(f.state.active.get_untracked().is_none());
    assert_eq!(
        f.vm.route(),
        Route::Extension {
            app: APP.into(),
            path: "task/1".into()
        }
    );
    assert_eq!(
        f.requests
            .borrow()
            .iter()
            .filter(|request| matches!(request, HostRequest::AlarmAlert { active: false, .. }))
            .count(),
        1
    );
    f.vm.navigate(Route::Start);
    f.ui.locked.set(true);
    f.state.open_app_reminder(&f.vm, &f.ui, &item);
    assert_eq!(
        f.vm.route(),
        Route::Start,
        "a retained dismissed callback cannot open again"
    );
    assert!(f.ui.locked.get_untracked());
}

#[test]
fn synchronous_stop_audio_cannot_open_into_a_replaced_document_session_or_host() {
    // Every action runs inside the host stop callback issued by dismiss(). The
    // same app/path remains registered, so ordinary AppContext admission alone
    // would accept the stale click after returning from the callback.
    for action in 0..6 {
        let f = Fixture::new();
        let item = f.fire();
        let vm = f.vm.clone();
        let state = f.state.clone();
        let current_registry = f.registry.clone();
        let next_registry = registry(replacement);
        let next_session = Rc::new(RefCell::new(None));
        let retained_session = next_session.clone();
        let ran = Rc::new(Cell::new(false));
        let ran_callback = ran.clone();
        *f.on_stop.borrow_mut() = Some(Box::new(move || {
            ran_callback.set(true);
            match action {
                0 => {
                    let data = vm.data().clone();
                    vm.replace_data(data).unwrap();
                }
                1 => {
                    let ui = Rc::new(Session::new(vm.now(), current_registry));
                    phone_sdk::set_current_session(&ui);
                    *retained_session.borrow_mut() = Some(ui);
                }
                2 => state.host_visibility(&vm, false),
                3 => state.host_closing(&vm),
                4 => state.tick_with_registry(&vm, &next_registry),
                5 => vm.set_extension_enabled(APP, false).unwrap(),
                _ => unreachable!(),
            }
        }));
        f.state.open_app_reminder(&f.vm, &f.ui, &item);
        assert!(ran.get(), "stop-audio action {action} did not execute");
        assert_eq!(
            f.vm.route(),
            Route::Start,
            "stale activation after action {action}"
        );
        assert!(
            f.ui.locked.get_untracked(),
            "stale unlock after action {action}"
        );
    }
}

#[test]
fn a_provider_registry_replacement_cannot_publish_the_previous_sources_snapshot() {
    let f = Fixture::new();
    let next = registry(replacement);
    let vm = f.vm.clone();
    let state = f.state.clone();
    let bound = next.clone();
    let tag = (vm.document_epoch(), vm.revision());
    COLLECT.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            state.tick_with_registry(&vm, &bound);
        }))
    });
    f.state.tick_with_registry(&f.vm, &f.registry);
    assert_eq!((f.vm.document_epoch(), f.vm.revision()), tag);
    assert!(f.state.active.get_untracked().is_none());
    assert!(
        f.state.app_reminders.borrow().tag.is_none(),
        "the old snapshot must not occupy the replacement registry's cache tag"
    );
    f.state.tick_with_registry(&f.vm, &next);
    let Some(Alert::App(item)) = f.state.active.get_untracked() else {
        panic!("the replacement provider should run on the next tick");
    };
    assert_eq!(item.reminder.title, "Replacement");
}

#[test]
fn closing_during_provider_collection_cannot_repopulate_the_retired_schedule() {
    let f = Fixture::new();
    let vm = f.vm.clone();
    let state = f.state.clone();
    COLLECT.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            state.host_closing(&vm);
        }))
    });
    f.state.tick_with_registry(&f.vm, &f.registry);
    assert!(f.state.active.get_untracked().is_none());
    assert!(f.state.app_reminders.borrow().tag.is_none());
    assert!(!f.requests.borrow().iter().any(|request| matches!(
        request,
        HostRequest::AlarmAlert { active: true, .. } | HostRequest::Notify { .. }
    )));
}
