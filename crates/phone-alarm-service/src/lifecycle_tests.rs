use super::*;
use phone_domain::PhoneData;
use phone_state::{HostEnvironment, HostPermission, HostServices};

struct Fixture {
    state: AlarmState,
    vm: Rc<PhoneViewModel>,
    calls: Rc<RefCell<Vec<HostRequest>>>,
    _owner: Owner,
}

impl Fixture {
    fn new() -> Self {
        let mut data = PhoneData::default();
        data.alarms = vec![Alarm {
            id: 7,
            label: "Morning".into(),
            hour: 7,
            enabled: true,
            weekdays: vec![0],
            ..Default::default()
        }];
        data.settings.sounds = true;
        data.settings.ringer_volume = 7;
        data.settings.vibration = true;
        let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
        vm.set_clock(phone_domain::days_from_date(2026, 11, 23) * 86_400 + 7 * 3_600);
        vm.set_utc_offset(0);
        vm.set_environment(HostEnvironment {
            notification_permission: HostPermission::Granted,
            ..Default::default()
        });
        let calls = Rc::new(RefCell::new(Vec::new()));
        let recorded = calls.clone();
        vm.set_services(HostServices {
            request: Rc::new(move |request| recorded.borrow_mut().push(request)),
        });
        let owner = Owner::new();
        let state = owner.with(AlarmState::new);
        Self {
            state,
            vm,
            calls,
            _owner: owner,
        }
    }

    fn audio(&self) -> Vec<(bool, u8, bool)> {
        self.calls
            .borrow()
            .iter()
            .filter_map(|request| match request {
                HostRequest::AlarmAlert {
                    active,
                    volume,
                    vibration,
                } => Some((*active, *volume, *vibration)),
                _ => None,
            })
            .collect()
    }

    fn notices(&self) -> usize {
        self.calls
            .borrow()
            .iter()
            .filter(|request| matches!(request, HostRequest::Notify { .. }))
            .count()
    }
}

#[test]
fn suspension_stops_audio_and_resume_reconciles_the_existing_alert_once() {
    let f = Fixture::new();
    f.state.tick(&f.vm);
    let active = f.state.active.get_untracked();
    assert!(active.is_some());
    assert_eq!(f.audio(), [(true, 7, true)]);
    assert_eq!(f.notices(), 1);

    f.state.host_visibility(&f.vm, false);
    f.state.host_visibility(&f.vm, false);
    f.state.tick(&f.vm);
    f.state.fire_alarm(&f.vm, 7);
    assert_eq!(f.audio(), [(true, 7, true), (false, 0, false)]);
    assert!(f.state.active.get_untracked() == active);

    f.state.host_visibility(&f.vm, true);
    assert_eq!(
        f.audio().len(),
        2,
        "visibility alone must not replay stale audio"
    );
    f.state.tick(&f.vm);
    f.state.tick(&f.vm);
    assert_eq!(
        f.audio(),
        [(true, 7, true), (false, 0, false), (true, 7, true)]
    );
    assert_eq!(
        f.notices(),
        1,
        "resuming an existing alert is not a new notification"
    );
    assert!(f.state.active.get_untracked() == active);
    assert!(!f.calls.borrow().iter().any(|request| matches!(
        request,
        HostRequest::PlayMedia { .. } | HostRequest::MediaControl { .. }
    )));
}

#[test]
fn hidden_dismiss_and_schedule_changes_cannot_recreate_an_alert_on_resume() {
    for remove in [false, true] {
        let f = Fixture::new();
        f.state.tick(&f.vm);
        f.state.host_visibility(&f.vm, false);
        if remove {
            f.vm.delete_alarm(7);
        } else {
            f.state.dismiss(&f.vm);
        }
        f.state.host_visibility(&f.vm, true);
        f.state.tick(&f.vm);
        assert!(f.state.active.get_untracked().is_none());
        assert_eq!(f.audio().iter().filter(|audio| audio.0).count(), 1);
        assert_eq!(f.notices(), 1);
    }
}

#[test]
fn a_snooze_retains_its_deadline_and_does_not_fire_while_hidden() {
    let f = Fixture::new();
    f.state.tick(&f.vm);
    let now = f.vm.now();
    f.state.snooze(&f.vm, 5);
    f.state.host_visibility(&f.vm, false);
    f.vm.set_clock(now + 299);
    f.state.tick(&f.vm);
    f.state.host_visibility(&f.vm, true);
    f.state.tick(&f.vm);
    assert!(f.state.active.get_untracked().is_none());
    assert_eq!(f.audio().iter().filter(|audio| audio.0).count(), 1);
    f.state.host_visibility(&f.vm, false);
    f.vm.set_clock(now + 300);
    f.state.tick(&f.vm);
    assert!(f.state.active.get_untracked().is_none());
    f.state.host_visibility(&f.vm, true);
    f.state.tick(&f.vm);
    assert!(f.state.active.get_untracked().is_some());
    assert_eq!(f.audio().iter().filter(|audio| audio.0).count(), 2);
}

#[test]
fn close_stops_before_teardown_and_old_callbacks_cannot_reopen_the_alert() {
    let f = Fixture::new();
    f.state.tick(&f.vm);
    f.state.host_closing(&f.vm);
    let count = f.calls.borrow().len();
    f.state.host_closing(&f.vm);
    f.state.host_visibility(&f.vm, true);
    f.state.fire_alarm(&f.vm, 7);
    f.state.tick(&f.vm);
    assert!(f.state.active.get_untracked().is_none());
    assert_eq!(f.audio(), [(true, 7, true), (false, 0, false)]);
    assert_eq!(f.calls.borrow().len(), count);
}
