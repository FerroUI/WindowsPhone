use phone_domain::{Action, AppId, CallRecord, PhoneData};
use phone_host::{platform::*, HostPlatform};
use phone_state::PhoneViewModel;
use std::{cell::Cell, rc::Rc};

fn activity(revision: u64, count: u32) -> DeviceResponse {
    DeviceResponse::MissedCalls(MissedCallActivity {
        revision,
        count,
        acknowledgement: (count > 0).then(|| MissedCallAcknowledgement {
            token: format!("receipt-{revision}"),
            count: count.min(500) as u16,
        }),
    })
}
fn granted() -> DeviceSnapshot {
    DeviceSnapshot {
        operations: vec![
            DeviceOperation::ReadMissedCalls,
            DeviceOperation::AcknowledgeMissedCalls,
        ],
        ..Default::default()
    }
}

#[test]
fn native_badges_are_transient_monotonic_and_retire_with_authority() {
    let writes = Rc::new(Cell::new(0));
    let observed = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::demo(),
        Rc::new(move |_| observed.set(observed.get() + 1)),
    );
    let original = vm.data().to_json().unwrap();
    vm.set_platform_state(PlatformState {
        platform: HostPlatform::Android,
        features: vec![HostFeature::Telephony],
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        vm.app_badge(AppId::Phone),
        0,
        "Native hosts never show sample missed calls"
    );
    vm.set_device_response(DeviceResponse::Snapshot(granted()))
        .unwrap();
    vm.set_device_response(activity(4, 700)).unwrap();
    assert_eq!(
        vm.app_badge(AppId::Phone),
        700,
        "Total is independent of bounded history and acknowledgement pages"
    );
    vm.set_device_response(activity(6, 1)).unwrap();
    vm.set_device_response(activity(5, 700)).unwrap();
    assert_eq!(
        vm.app_badge(AppId::Phone),
        1,
        "Delayed pre-ack query cannot resurrect the count"
    );
    let mut readonly = granted();
    readonly
        .operations
        .retain(|op| *op != DeviceOperation::AcknowledgeMissedCalls);
    vm.set_device_response(DeviceResponse::Snapshot(readonly))
        .unwrap();
    assert!(vm
        .device_state()
        .missed_calls
        .unwrap()
        .acknowledgement
        .is_none());
    assert!(vm.set_device_response(activity(7, 1)).is_err());
    vm.set_device_response(DeviceResponse::Snapshot(DeviceSnapshot::default()))
        .unwrap();
    assert_eq!(vm.app_badge(AppId::Phone), 0);
    assert!(vm.device_state().missed_calls.is_none());
    assert_eq!(vm.data().to_json().unwrap(), original);
    assert_eq!(writes.get(), 0);
}

#[test]
fn local_call_acknowledgement_preserves_direction_and_later_calls() {
    let legacy: CallRecord = serde_json::from_str(
        r#"{"id":11,"name":"A","number":"5550101","timestamp":100,"incoming":true,"missed":true}"#,
    )
    .unwrap();
    assert!(!legacy.acknowledged);
    let mut data = PhoneData::default();
    data.call_history.push(legacy);
    let writes = Rc::new(Cell::new(0));
    let observed = writes.clone();
    let vm = PhoneViewModel::new(data, Rc::new(move |_| observed.set(observed.get() + 1)));
    let captured = vec![11];
    vm.dispatch(Action::AddCall(CallRecord {
        id: 12,
        name: "B".into(),
        number: "5550102".into(),
        timestamp: 101,
        incoming: true,
        missed: true,
        ..Default::default()
    }));
    vm.dispatch(Action::AcknowledgeCalls(captured.clone()));
    assert_eq!(vm.app_badge(AppId::Phone), 1);
    assert!(vm.data().call_history.iter().all(|call| call.missed));
    assert!(
        vm.data()
            .call_history
            .iter()
            .find(|call| call.id == 11)
            .unwrap()
            .acknowledged
    );
    assert!(
        !vm.data()
            .call_history
            .iter()
            .find(|call| call.id == 12)
            .unwrap()
            .acknowledged
    );
    let before = writes.get();
    vm.dispatch(Action::AcknowledgeCalls(captured));
    assert_eq!(writes.get(), before, "Repeated acknowledgement is a no-op");
    let restored = PhoneData::from_json(&vm.data().to_json().unwrap()).unwrap();
    assert_eq!(restored.badge(AppId::Phone), 1);
}
