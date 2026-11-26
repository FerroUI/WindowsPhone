use super::*;
use model::{Fields, Priority, Reminder};
use phone_domain::{AppId, PhoneData, Route};
use phone_sdk::AppRegistry;

fn fixture() -> (Rc<PhoneViewModel>, Rc<Session>) {
    let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
    vm.set_clock(phone_domain::days_from_date(2026, 11, 26) * 86_400 + 23 * 3600 + 59 * 60);
    vm.set_utc_offset(0);
    let registry = Rc::new(AppRegistry::new(vec![crate::registration()]).unwrap());
    registry.bind(&vm);
    let ui = Rc::new(Session::new(vm.now(), registry));
    (vm, ui)
}
fn fields() -> Fields {
    Fields {
        title: "Return library books".into(),
        due: Some("2026-11-27".into()),
        priority: Priority::High,
        reminder: Some(Reminder {
            date: "2026-11-26".into(),
            time: "23:59".into(),
        }),
        ..Default::default()
    }
}

#[test]
fn typed_task_paths_and_activation_do_not_alias_deleted_or_noncanonical_identifiers() {
    let (vm, ui) = fixture();
    let task = store::save(&vm, vm.document_epoch(), None, fields()).unwrap();
    for path in [
        "task/0",
        "task/01",
        "task/+1",
        "task/1/extra",
        "task/4294967295",
        "unknown",
    ] {
        assert!(launch_path(path).is_err(), "{path}");
    }
    activate(&vm, &ui, launch_path("tasks").unwrap()).unwrap();
    assert!(ui.state::<State>().active.get_untracked());
    activate(&vm, &ui, launch_path("task/1").unwrap()).unwrap();
    assert_eq!(ui.state::<State>().selected.get_untracked(), Some(task.id));
    assert!(!crate::can_swipe(&ui));
    vm.navigate(Route::App(AppId::Calendar));
    commands::pin(&vm, &ui, vm.document_epoch(), &task);
    assert_eq!(
        vm.route(),
        Route::Start,
        "pinning returns to the new Start tile"
    );
    assert!(vm
        .data()
        .app_links
        .iter()
        .any(|link| link.app == "phone.calendar" && link.path == "task/1"));
    store::delete(&vm, vm.document_epoch(), &[task]).unwrap();
    assert!(activate(&vm, &ui, launch_path("task/1").unwrap()).is_err());
    assert!(activate(
        &vm,
        &ui,
        serde_json::json!({"calendar":"task","id":1,"extra":true})
    )
    .is_err());
}

#[test]
fn unsaved_editor_survives_app_switch_and_cancel_until_explicit_discard_and_reset_invalidates_callbacks(
) {
    let (vm, ui) = fixture();
    let state = ui.state::<State>();
    state.begin(&vm, None);
    assert!(!state.dirty());
    state.subject.set("keep this draft".into());
    assert!(state.dirty());
    let old = state.editor.get_untracked().unwrap();
    // Session-owned draft state outlives the active route; only ephemeral menu
    // state is dismissed by Calendar's route/deactivate callbacks.
    ui.registry()
        .route_changed(&vm, &ui, &Route::App(AppId::Calendar), &Route::Start);
    assert_eq!(state.subject.get_untracked(), "keep this draft");
    assert!(state.editor.get_untracked().is_some());
    assert!(activate(&vm, &ui, launch_path("tasks").unwrap()).is_err());
    assert!(back(&vm, &ui));
    let discard = ui.dialog.get_untracked().unwrap();
    ui.dialog.set(None);
    assert!(state.editor.get_untracked().is_some());
    state.reset();
    state.begin(&vm, None);
    state.subject.set("new draft".into());
    (discard.save)(Vec::new());
    assert_eq!(state.subject.get_untracked(), "new draft");
    assert!(state.editor.get_untracked().is_some());
    assert!(!state.matches(&vm, &old));
    assert!(back(&vm, &ui));
    let discard = ui.dialog.get_untracked().unwrap();
    // Match the shared DialogAction transaction: clear feedback, invoke the
    // callback, then release the successful dialog. Calling only `save` would
    // retain its Rc<Session> inside that Session's dialog signal until TLS drop.
    ui.form_error.set(String::new());
    (discard.save)(Vec::new());
    assert!(ui.form_error.get_untracked().is_empty());
    ui.dialog.set(None);
    assert!(state.editor.get_untracked().is_none());
}

#[test]
fn provider_uses_persisted_tasks_while_calendar_is_inactive_and_tracks_completion_edits_and_epoch()
{
    let (vm, ui) = fixture();
    let task = store::save(&vm, vm.document_epoch(), None, fields()).unwrap();
    assert_eq!(vm.route(), Route::Start);
    let snapshot = ui.registry().reminder_snapshot(&vm);
    assert!(snapshot.errors.is_empty());
    assert_eq!(snapshot.reminders.len(), 1);
    assert_eq!(snapshot.reminders[0].reminder.path, "task/1");
    assert_eq!(snapshot.reminders[0].document_epoch, vm.document_epoch());
    let mut changed = task.fields.clone();
    changed.title = "Edited subject".into();
    let edited = store::save(&vm, vm.document_epoch(), Some(&task), changed).unwrap();
    let reminder = reminders(&vm).unwrap().remove(0);
    assert_eq!(reminder.title, "Edited subject");
    assert_eq!(reminder.revision, 2);
    let completed = store::complete(&vm, vm.document_epoch(), &edited, true).unwrap();
    assert!(reminders(&vm).unwrap().is_empty());
    let reopened = store::complete(&vm, vm.document_epoch(), &completed, false).unwrap();
    assert_eq!(reminders(&vm).unwrap().len(), 1);
    store::delete(&vm, vm.document_epoch(), &[reopened]).unwrap();
    assert!(reminders(&vm).unwrap().is_empty());
    vm.replace_data(PhoneData::default()).unwrap();
    assert!(ui.registry().reminder_snapshot(&vm).reminders.is_empty());
}

#[test]
fn reminder_editor_defaults_cross_midnight_without_scheduling_in_the_past() {
    let (vm, ui) = fixture();
    let state = ui.state::<State>();
    state.begin(&vm, None);
    state.subject.set("midnight".into());
    state.reminder_enabled.set(true);
    let fields = state.fields().unwrap();
    let reminder = fields.reminder.unwrap();
    assert_eq!(reminder.date, "2026-11-27");
    assert_eq!(reminder.time, "00:00");
    assert_eq!(reminder.local_timestamp().unwrap(), vm.local_now() + 60);
    state.reminder_time.set(None);
    assert!(state.fields().is_err());
    assert!(state.dirty());
}
