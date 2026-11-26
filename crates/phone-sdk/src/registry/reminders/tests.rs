use super::*;
use crate::{AppManifest, AppRegistration};
use ferroui_view::prelude::Element;
use phone_controls::Icon;
use phone_domain::{AppId, PhoneData, Route};
use std::rc::Rc;

fn reminder(key: &str) -> AppReminder {
    AppReminder {
        key: key.into(),
        revision: 1,
        scheduled_local: days_from_date(2026, 11, 26) * 86_400,
        title: "Return library books".into(),
        detail: "Local task".into(),
        path: "task/1".into(),
    }
}
fn provide(_: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
    Ok(vec![reminder("z"), reminder("a")])
}
fn entry(id: &'static str) -> AppRegistration {
    AppRegistration::new(
        AppManifest::new(id, "Tasks", Icon::Calendar).capabilities(&[Capability::Notifications]),
        |_| Element::Empty,
    )
    .with_reminders(provide)
}
fn model() -> Rc<PhoneViewModel> {
    PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}))
}

#[test]
fn all_enabled_providers_run_outside_the_active_route_in_stable_owner_key_order() {
    let vm = model();
    let registry = AppRegistry::new(vec![entry("test.z"), entry("test.a")]).unwrap();
    registry.bind(&vm);
    assert_eq!(vm.route(), Route::Start);
    let snapshot = registry.reminder_snapshot(&vm);
    assert!(snapshot.errors.is_empty());
    assert_eq!(
        snapshot
            .reminders
            .iter()
            .map(|item| item.identity())
            .collect::<Vec<_>>(),
        [
            ("test.a", "a"),
            ("test.a", "z"),
            ("test.z", "a"),
            ("test.z", "z")
        ]
    );
    vm.set_extension_enabled("test.a", false).unwrap();
    let snapshot = registry.reminder_snapshot(&vm);
    assert_eq!(snapshot.reminders.len(), 2);
    assert!(snapshot
        .reminders
        .iter()
        .all(|item| item.app_id == "test.z"));
}

#[test]
fn providers_without_notification_capability_and_uninstalled_builtins_are_excluded() {
    let vm = model();
    let no_capability = AppRegistration::new(
        AppManifest::new("test.no-capability", "No", Icon::Calendar),
        |_| Element::Empty,
    )
    .with_reminders(provide);
    let calendar = AppRegistration::builtin(AppId::Calendar, |_, _| Element::Empty)
        .with_launch_path(|_| Ok(serde_json::json!({"task":1})))
        .with_activation(|_, _, _| Ok(()))
        .with_reminders(provide);
    let registry = AppRegistry::new(vec![no_capability, calendar]).unwrap();
    registry.bind(&vm);
    assert_eq!(registry.reminder_snapshot(&vm).reminders.len(), 2);
    let mut replaced = vm.data().clone();
    replaced.installed.retain(|app| *app != AppId::Calendar);
    vm.replace_data(replaced).unwrap();
    assert!(registry.reminder_snapshot(&vm).reminders.is_empty());
}

#[test]
fn duplicate_invalid_or_oversized_provider_schedules_are_rejected_atomically() {
    fn duplicate(_: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
        Ok(vec![reminder("same"), reminder("same")])
    }
    fn invalid(_: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
        let mut bad = reminder("bad");
        bad.scheduled_local += 1;
        Ok(vec![reminder("valid"), bad])
    }
    fn over_limit(_: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
        Ok((0..=MAX_PROVIDER_REMINDERS)
            .map(|index| reminder(&index.to_string()))
            .collect())
    }
    for provider in [duplicate as crate::ReminderProvider, invalid, over_limit] {
        let vm = model();
        let registry = AppRegistry::new(vec![
            entry("test.good"),
            entry("test.bad").with_reminders(provider),
        ])
        .unwrap();
        registry.bind(&vm);
        let before = vm.export_json().unwrap();
        let snapshot = registry.reminder_snapshot(&vm);
        assert_eq!(snapshot.reminders.len(), 2);
        assert!(snapshot
            .reminders
            .iter()
            .all(|item| item.app_id == "test.good"));
        assert_eq!(snapshot.errors.len(), 1);
        assert_eq!(snapshot.errors[0].0, "test.bad");
        assert_eq!(vm.export_json().unwrap(), before);
    }
}

#[test]
fn builtin_reminder_paths_pass_the_existing_typed_activation_boundary() {
    let vm = model();
    let registry = AppRegistry::new(vec![AppRegistration::builtin(AppId::Calendar, |_, _| {
        Element::Empty
    })
    .with_launch_path(|path| {
        if path == "task/2" {
            Ok(serde_json::json!({"task":2}))
        } else {
            Err("Unknown task path.".into())
        }
    })
    .with_activation(|_, _, _| Ok(()))
    .with_reminders(provide)])
    .unwrap();
    registry.bind(&vm);
    let snapshot = registry.reminder_snapshot(&vm);
    assert!(snapshot.reminders.is_empty());
    assert_eq!(snapshot.errors[0].1, "Unknown task path.");
}

#[test]
fn reentrant_document_replacement_invalidates_the_entire_collected_snapshot() {
    fn replacing(vm: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
        vm.replace_data(PhoneData::demo())?;
        provide(vm)
    }
    let vm = model();
    let registry = AppRegistry::new(vec![
        entry("test.a"),
        entry("test.z").with_reminders(replacing),
    ])
    .unwrap();
    registry.bind(&vm);
    let epoch = vm.document_epoch();
    let snapshot = registry.reminder_snapshot(&vm);
    assert_eq!(snapshot.document_epoch, epoch);
    assert_ne!(snapshot.document_epoch, vm.document_epoch());
    assert!(snapshot.reminders.is_empty());
    assert!(snapshot
        .errors
        .iter()
        .any(|(_, message)| message.contains("document changed")));
}
