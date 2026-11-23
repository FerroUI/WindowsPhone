use phone_domain::{AppId, PhoneData, Route};
use phone_state::{HostEnvironment, PhoneViewModel};
use std::{cell::Cell, rc::Rc};

#[test]
fn failed_restore_preserves_document_navigation_identity_and_storage_callback() {
    let writes = Rc::new(Cell::new(0));
    let saved = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::demo(),
        Rc::new(move |_| saved.set(saved.get() + 1)),
    );
    vm.navigate(Route::App(AppId::People));
    let before = vm.data().clone();
    let revision = vm.revision();
    let epoch = vm.document_epoch();
    let recent = vm.recent_apps();
    let mut replacement = before.clone();
    replacement.settings.phone_name = "restored phone".into();
    let rejected = vm.replace_data_after_save(replacement.clone(), |_| Err("disk full".into()));
    assert_eq!(rejected.unwrap_err(), "disk full");
    assert_eq!(*vm.data(), before);
    assert_eq!(vm.route(), Route::App(AppId::People));
    assert_eq!(vm.recent_apps(), recent);
    assert_eq!(vm.revision(), revision);
    assert_eq!(vm.document_epoch(), epoch);
    assert_eq!(writes.get(), 0);

    let transactions = Cell::new(0);
    vm.replace_data_after_save(replacement, |data| {
        assert_eq!(data.settings.phone_name, "restored phone");
        transactions.set(transactions.get() + 1);
        Ok(())
    })
    .unwrap();
    assert_eq!(transactions.get(), 1);
    assert_eq!(writes.get(), 0); // The host's successful transaction is not repeated.
    assert_eq!(vm.document_epoch(), epoch + 1);
    assert_eq!(vm.route(), Route::Start);
    assert!(vm.recent_apps().is_empty());
}

#[test]
fn observed_host_power_saver_pauses_motion_without_changing_phone_preferences() {
    let writes = Rc::new(Cell::new(0));
    let saved = writes.clone();
    let vm = PhoneViewModel::new(
        PhoneData::demo(),
        Rc::new(move |_| saved.set(saved.get() + 1)),
    );
    let before = vm.data().clone();
    assert!(vm.motion_enabled());
    vm.set_environment(HostEnvironment {
        power_saver: Some(true),
        ..Default::default()
    });
    assert!(!vm.motion_enabled());
    assert!(!vm.live_tiles_enabled());
    assert_eq!(*vm.data(), before);
    vm.set_environment(HostEnvironment::default());
    assert!(vm.motion_enabled());
    assert_eq!(writes.get(), 0);
}
