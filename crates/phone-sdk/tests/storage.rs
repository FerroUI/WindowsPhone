use ferroui_view::prelude::*;
use phone_controls::Icon;
use phone_domain::{PhoneData, Route, MAX_APP_NAMESPACES, MAX_APP_STATE_BYTES};
use phone_sdk::{session::Session, AppManifest, AppRegistration, AppRegistry, AppStorage};
use phone_state::PhoneViewModel;
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

fn fixture(data: PhoneData) -> (Rc<PhoneViewModel>, Rc<Cell<usize>>) {
    let writes = Rc::new(Cell::new(0));
    let saved = writes.clone();
    let vm = PhoneViewModel::new(data, Rc::new(move |_| saved.set(saved.get() + 1)));
    (vm, writes)
}

#[test]
fn namespaced_storage_keeps_other_apps_and_ignores_equal_writes() {
    let (vm, writes) = fixture(PhoneData::default());
    let first = AppStorage::new(vm.clone(), "tests.first");
    let second = AppStorage::new(vm.clone(), "tests.second");
    first
        .write(&json!({"board": [1, 2], "version": 1}))
        .unwrap();
    second.write(&json!({"document": "kept"})).unwrap();
    let revision = vm.revision();
    let data = vm.data().clone();
    first
        .write(&json!({"version": 1, "board": [1, 2]}))
        .unwrap();
    AppStorage::new(vm.clone(), "tests.absent").clear().unwrap();
    assert_eq!(writes.get(), 2);
    assert_eq!(vm.revision(), revision);
    assert_eq!(*vm.data(), data);
    first.clear().unwrap();
    assert_eq!(writes.get(), 3);
    assert_eq!(first.read::<Value>().unwrap(), Value::Null);
    assert_eq!(second.read::<Value>().unwrap(), json!({"document": "kept"}));
}

#[test]
fn rejected_keys_sizes_and_namespace_overflow_do_not_commit_partial_data() {
    let (vm, writes) = fixture(PhoneData::default());
    let before = vm.data().clone();
    assert!(AppStorage::new(vm.clone(), "../escape").write(&42).is_err());
    assert!(AppStorage::new(vm.clone(), "tests.large")
        .write(&"x".repeat(MAX_APP_STATE_BYTES))
        .is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
    let mut full = PhoneData::default();
    for index in 0..MAX_APP_NAMESPACES {
        full.app_data
            .insert(format!("tests.app{index}"), json!(index));
    }
    let (vm, writes) = fixture(full);
    let before = vm.data().clone();
    assert!(AppStorage::new(vm.clone(), "tests.overflow")
        .write(&1)
        .is_err());
    assert_eq!(*vm.data(), before);
    assert_eq!(writes.get(), 0);
}

#[test]
fn nested_namespace_write_during_save_is_rejected_without_losing_the_outer_commit() {
    let slot: Rc<RefCell<Weak<PhoneViewModel>>> = Rc::new(RefCell::new(Weak::new()));
    let callback_slot = slot.clone();
    let rejected = Rc::new(Cell::new(false));
    let nested = rejected.clone();
    let vm = PhoneViewModel::new(
        PhoneData::default(),
        Rc::new(move |_| {
            let vm = callback_slot.borrow().upgrade().unwrap();
            nested.set(AppStorage::new(vm, "tests.nested").write(&true).is_err());
        }),
    );
    *slot.borrow_mut() = Rc::downgrade(&vm);
    AppStorage::new(vm.clone(), "tests.outer")
        .write(&json!({"saved": true}))
        .unwrap();
    assert!(rejected.get());
    assert_eq!(vm.data().app_data.len(), 1);
    assert_eq!(vm.app_data("tests.outer"), Some(json!({"saved": true})));
}

struct Transient {
    tick: Signal<i64>,
    draft: Signal<String>,
}
impl Default for Transient {
    fn default() -> Self {
        Self {
            tick: signal(0),
            draft: signal("unsaved".into()),
        }
    }
}
fn runtime(vm: &Rc<PhoneViewModel>) -> (Rc<AppRegistry>, Rc<Session>) {
    let app = AppRegistration::new(
        AppManifest::new("tests.clock", "Clock", Icon::Timer),
        |_| Element::Empty,
    )
    .with_tick(|_, ui, now| ui.state::<Transient>().tick.set(now))
    .with_reset(|ui| ui.state::<Transient>().draft.set(String::new()));
    let registry = Rc::new(AppRegistry::new(vec![app]).unwrap());
    vm.set_registered_apps(["tests.clock".into()]);
    let ui = Rc::new(Session::new(0, registry.clone()));
    (registry, ui)
}

#[test]
fn app_clock_ticks_do_not_change_document_revision_or_persistence() {
    let (vm, writes) = fixture(PhoneData::default());
    let (registry, ui) = runtime(&vm);
    vm.navigate(Route::Extension {
        app: "tests.clock".into(),
        path: "".into(),
    });
    let before = vm.data().clone();
    let revision = vm.revision();
    for tick in 1..=10 {
        registry.tick(&vm, &ui, tick);
    }
    assert_eq!(ui.state::<Transient>().tick.get_untracked(), 10);
    assert_eq!(*vm.data(), before);
    assert_eq!(vm.revision(), revision);
    assert_eq!(writes.get(), 0);
}

#[test]
fn failed_restore_preserves_app_namespace_and_draft_then_success_can_reset_transients() {
    let (vm, writes) = fixture(PhoneData::default());
    let (registry, ui) = runtime(&vm);
    let storage = AppStorage::new(vm.clone(), "tests.clock");
    storage.write(&json!({"saved": 1})).unwrap();
    let route = Route::Extension {
        app: "tests.clock".into(),
        path: "draft".into(),
    };
    vm.navigate(route.clone());
    let state = ui.state::<Transient>();
    let epoch = vm.document_epoch();
    let mut restored = vm.data().clone();
    restored
        .app_data
        .insert("tests.clock".into(), json!({"saved": 2}));
    assert!(vm
        .replace_data_after_save(restored.clone(), |_| Err("quota".into()))
        .is_err());
    assert_eq!(storage.read::<Value>().unwrap(), json!({"saved": 1}));
    assert_eq!(vm.route(), route);
    assert_eq!(vm.document_epoch(), epoch);
    assert_eq!(state.draft.get_untracked(), "unsaved");
    assert_eq!(writes.get(), 1);
    vm.replace_data_after_save(restored, |_| Ok(())).unwrap();
    registry.reset(&vm, &ui);
    assert_eq!(storage.read::<Value>().unwrap(), json!({"saved": 2}));
    assert_eq!(vm.document_epoch(), epoch + 1);
    assert_eq!(state.draft.get_untracked(), "");
    assert_eq!(
        writes.get(),
        1,
        "the explicit host restore save is not duplicated"
    );
}
