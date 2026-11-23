use super::*;
use phone_domain::{Alarm, AppId, PhoneData};
use phone_sdk::{AppRegistration, AppRegistry};
use phone_state::{HostRequest, HostServices};

#[test]
fn host_lifecycle_stops_and_reconciles_alarm_before_releasing_the_shell() {
    let mut data = PhoneData::default();
    data.alarms = vec![Alarm {
        id: 7,
        hour: 7,
        enabled: true,
        weekdays: vec![0],
        ..Default::default()
    }];
    let vm = PhoneViewModel::new(data, Rc::new(|_| {}));
    vm.set_clock(phone_domain::days_from_date(2026, 11, 23) * 86_400 + 7 * 3_600);
    vm.set_utc_offset(0);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = calls.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| recorded.borrow_mut().push(request)),
    });
    let registry = Rc::new(
        AppRegistry::new(vec![AppRegistration::builtin(AppId::Alarms, |_, _| {
            Element::Empty
        })])
        .unwrap(),
    );
    let ui = Rc::new(session::Session::new(vm.now(), registry));
    MODEL.with(|slot| *slot.borrow_mut() = Rc::downgrade(&vm));
    SESSION.with(|slot| *slot.borrow_mut() = Rc::downgrade(&ui));
    let alarm = ui.state::<phone_alarm_service::AlarmState>();
    alarm_tick(&vm);
    host_visibility(false);
    alarm_tick(&vm);
    host_visibility(true);
    alarm_tick(&vm);
    host_closing(&vm);
    host_visibility(true);
    alarm_tick(&vm);
    // A retained app state cannot revive alerts after its shell has closed.
    alarm.fire_alarm(&vm, 7);
    alarm.tick(&vm);
    let active: Vec<_> = calls
        .borrow()
        .iter()
        .filter_map(|request| match request {
            HostRequest::AlarmAlert { active, .. } => Some(*active),
            _ => None,
        })
        .collect();
    assert_eq!(active, [true, false, true, false]);
    assert!(!host_ready(&vm));
    assert!(alarm.active.get_untracked().is_none());
    assert!(!calls.borrow().iter().any(|request| matches!(
        request,
        HostRequest::PlayMedia { .. } | HostRequest::MediaControl { .. }
    )));
}
