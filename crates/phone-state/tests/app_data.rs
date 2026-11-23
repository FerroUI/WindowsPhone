use phone_domain::{validate_app_value, PhoneData, MAX_APP_STATE_BYTES};
use phone_state::PhoneViewModel;
use serde_json::{json, Value};
use std::{cell::Cell, rc::Rc};

#[test]
fn namespace_transactions_roundtrip_and_reject_invalid_values_atomically() {
    let saves = Rc::new(Cell::new(0));
    let counter = saves.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| counter.set(counter.get() + 1)),
    );
    let expected = json!({"version":1,"max":9_007_199_254_740_991_u64,"min":-9_007_199_254_740_991_i64,"large_id":"18446744073709551615"});
    vm.set_app_data("tests.data", Some(expected.clone()))
        .unwrap();
    assert_eq!(saves.get(), 1);
    let restored = PhoneData::from_json(&vm.export_json().unwrap()).unwrap();
    assert_eq!(restored.app_data.get("tests.data"), Some(&expected));
    for invalid in [
        json!(9_007_199_254_740_992_u64),
        json!(-9_007_199_254_740_992_i64),
        json!(u64::MAX),
        json!({"deep":[{"unsafe":u64::MAX}]}),
        json!("x".repeat(MAX_APP_STATE_BYTES)),
    ] {
        assert!(vm.set_app_data("tests.data", Some(invalid)).is_err());
        assert_eq!(vm.app_data("tests.data"), Some(expected.clone()));
        assert_eq!(saves.get(), 1);
    }
    assert!(vm.set_app_data("../invalid", Some(json!(1))).is_err());
    vm.set_app_data("tests.data", None).unwrap();
    assert_eq!(saves.get(), 2);
    assert!(vm.app_data("tests.data").is_none());
}

#[test]
fn depth_node_and_backup_validation_share_the_same_limits() {
    let mut nested = Value::Null;
    for _ in 0..32 {
        nested = Value::Array(vec![nested]);
    }
    assert!(validate_app_value(&nested).is_ok());
    nested = Value::Array(vec![nested]);
    assert!(validate_app_value(&nested).unwrap_err().contains("nesting"));
    assert!(validate_app_value(&Value::Array(vec![Value::Null; 16_383])).is_ok());
    assert!(validate_app_value(&Value::Array(vec![Value::Null; 16_384]))
        .unwrap_err()
        .contains("values"));
    let mut backup = PhoneData::default();
    backup.app_data.insert(
        "tests.data".into(),
        json!({"integer":9_007_199_254_740_992_u64}),
    );
    assert!(backup.normalize().unwrap_err().contains("exact integer"));
    let json = serde_json::to_string(&backup).unwrap();
    assert!(PhoneData::from_json(&json).is_err());
}

#[test]
fn actions_enqueued_by_an_app_save_keep_their_original_order() {
    use phone_domain::AppId;
    use std::cell::RefCell;
    use std::rc::Weak;
    let weak = Rc::new(RefCell::new(Weak::<PhoneViewModel>::new()));
    let receiver = weak.clone();
    let once = Cell::new(false);
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| {
            if !once.replace(true) {
                let vm = receiver.borrow().upgrade().unwrap();
                assert!(vm.set_app_data("tests.nested", Some(json!(true))).is_err());
                vm.uninstall(AppId::Weather);
                vm.install(AppId::Weather);
            }
        }),
    );
    *weak.borrow_mut() = Rc::downgrade(&vm);
    vm.set_app_data("tests.root", Some(json!(true))).unwrap();
    assert!(vm.data().installed.contains(&AppId::Weather));
    assert_eq!(vm.app_data("tests.root"), Some(json!(true)));
    assert_eq!(vm.app_data("tests.nested"), None);
}
