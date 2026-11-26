use super::*;
use phone_sdk::AppReminder;

fn now() -> i64 {
    phone_domain::days_from_date(2026, 11, 26) * 86_400 + 9 * 3600
}
fn item(key: &str, time: i64) -> ScopedAppReminder {
    ScopedAppReminder {
        app_id: "phone.calendar",
        app_title: "Calendar",
        builtin: Some(phone_domain::AppId::Calendar),
        document_epoch: 1,
        reminder: AppReminder {
            key: key.into(),
            revision: 1,
            scheduled_local: time,
            title: "Task".into(),
            detail: "Notes".into(),
            path: "task/1".into(),
        },
    }
}
fn snapshot(
    epoch: u64,
    revision: u64,
    mut reminders: Vec<ScopedAppReminder>,
) -> AppReminderSnapshot {
    for item in &mut reminders {
        item.document_epoch = epoch;
    }
    AppReminderSnapshot {
        document_epoch: epoch,
        document_revision: revision,
        reminders,
        errors: vec![],
    }
}

#[test]
fn due_minutes_use_current_offset_and_rapid_edits_do_not_redeliver() {
    let mut clock = Schedule::default();
    let original = item("one", now());
    clock.replace(snapshot(1, 1, vec![original.clone()]));
    assert_eq!(clock.poll(now() - 19_800, 19_800), [original.clone()]);
    let mut edited = original.clone();
    edited.reminder.revision += 1;
    edited.reminder.title = "Edited".into();
    clock.replace(snapshot(1, 2, vec![edited.clone()]));
    assert!(clock.poll(now() - 19_800 + 1, 19_800).is_empty());
    assert_eq!(clock.current(&original), Some(edited.clone()));
    clock.replace(snapshot(1, 3, vec![])); // completed, deleted or disabled
    assert!(clock.current(&original).is_none());
    assert!(clock.poll(now() - 19_800 + 2, 19_800).is_empty());
    clock.replace(snapshot(1, 4, vec![edited])); // rapid reopen/re-enable
    assert!(clock.poll(now() - 19_800 + 3, 19_800).is_empty());
}

#[test]
fn snoozes_refresh_content_but_cancel_on_reschedule_removal_and_epoch_change() {
    for change in ["content", "time", "removed", "epoch"] {
        let mut clock = Schedule::default();
        let original = item("one", now());
        clock.replace(snapshot(1, 1, vec![original.clone()]));
        assert_eq!(clock.poll(now(), 0).len(), 1);
        clock.snooze(&original, now(), 5);
        let mut edited = original.clone();
        edited.reminder.revision += 1;
        edited.reminder.title = "Current title".into();
        if change == "time" {
            edited.reminder.scheduled_local += 3600;
        }
        clock.replace(snapshot(
            if change == "epoch" { 2 } else { 1 },
            2,
            if change == "removed" {
                vec![]
            } else {
                vec![edited.clone()]
            },
        ));
        assert!(clock.poll(now() + 299, 3600).is_empty());
        let due = clock.poll(now() + 300, 3600);
        if change == "content" {
            assert_eq!(due, [edited]);
        } else {
            assert!(due.is_empty());
        }
    }
}

#[test]
fn no_missed_schedule_replay_and_distinct_app_keys_keep_deterministic_order() {
    let mut clock = Schedule::default();
    let first = item("a", now());
    let second = item("b", now());
    clock.replace(snapshot(
        1,
        1,
        vec![second.clone(), first.clone(), item("past", now() - 60)],
    ));
    assert_eq!(clock.poll(now(), 0), [first.clone(), second]);
    clock.replace(snapshot(1, 2, vec![first.clone()]));
    assert!(clock.poll(now() - 3600, 0).is_empty());
    assert!(
        clock.poll(now(), 0).is_empty(),
        "clock/DST rollback cannot replay a live item's fired instant"
    );
    clock.replace(snapshot(2, 3, vec![first]));
    assert_eq!(
        clock.poll(now(), 0).len(),
        1,
        "the replacement document owns a new schedule"
    );
}
