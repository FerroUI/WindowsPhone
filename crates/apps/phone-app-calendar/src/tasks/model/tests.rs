use super::*;

fn task(
    id: u32,
    title: &str,
    due: Option<&str>,
    priority: Priority,
    completed_at: Option<i64>,
) -> Task {
    Task {
        id,
        revision: 1,
        fields: Fields {
            title: title.into(),
            notes: format!("notes for {title}"),
            due: due.map(str::to_owned),
            priority,
            reminder: None,
        },
        created: 1,
        updated: 2,
        completed_at,
    }
}

#[test]
fn filtering_search_and_sort_preserve_task_identity_and_completion_order() {
    let library = Library {
        version: 1,
        next_id: 6,
        tasks: vec![
            task(1, "Call Ada", Some("2026-11-25"), Priority::Normal, None),
            task(2, "Library", Some("2026-11-26"), Priority::High, None),
            task(3, "旅行", None, Priority::Low, None),
            task(
                4,
                "Finished A",
                Some("2026-11-20"),
                Priority::High,
                Some(10),
            ),
            task(5, "Finished B", None, Priority::Normal, Some(20)),
        ],
    };
    library.validate().unwrap();
    let today = calendar_day("2026-11-26").unwrap();
    let ids = |filter, sort, query| {
        library
            .rows(filter, sort, query, today)
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(Filter::Active, Sort::DueDate, ""), [1, 2, 3]);
    assert_eq!(ids(Filter::Active, Sort::Priority, ""), [2, 1, 3]);
    assert_eq!(ids(Filter::Due, Sort::DueDate, ""), [1, 2]);
    assert_eq!(ids(Filter::Completed, Sort::DueDate, ""), [5, 4]);
    assert_eq!(ids(Filter::All, Sort::Subject, "NOTES FOR CALL"), [1]);
    assert_eq!(ids(Filter::All, Sort::Subject, "旅行"), [3]);
    assert!(library.task(1).unwrap().overdue(today));
    assert!(!library.task(4).unwrap().overdue(today));
}

#[test]
fn rescheduling_uses_calendar_days_across_leap_months_and_preserves_reminder_lead_time() {
    let fields = Fields {
        title: "submit report".into(),
        due: Some("2024-02-29".into()),
        reminder: Some(Reminder {
            date: "2024-02-28".into(),
            time: "09:35".into(),
        }),
        ..Default::default()
    };
    let postponed = fields
        .postponed(calendar_day("2024-02-29").unwrap())
        .unwrap();
    assert_eq!(postponed.due.as_deref(), Some("2024-03-01"));
    assert_eq!(postponed.reminder.as_ref().unwrap().date, "2024-02-29");
    assert_eq!(postponed.reminder.as_ref().unwrap().time, "09:35");
    let moved = postponed
        .due_today(calendar_day("2024-03-05").unwrap())
        .unwrap();
    assert_eq!(moved.reminder.as_ref().unwrap().date, "2024-03-04");
    assert_eq!(
        moved.reminder.unwrap().local_timestamp().unwrap(),
        calendar_day("2024-03-04").unwrap() * 86_400 + 575 * 60
    );
    let end = Fields {
        title: "last date".into(),
        due: Some("9999-12-31".into()),
        ..Default::default()
    };
    assert!(end.postponed(0).is_err());
}

#[test]
fn malformed_or_future_data_is_rejected_without_normalizing_user_content() {
    let mut library = Library {
        version: 1,
        next_id: 2,
        tasks: vec![task(1, "a", None, Priority::Normal, None)],
    };
    library.tasks.push(library.tasks[0].clone());
    assert!(library.validate().is_err());
    library.tasks.pop();
    library.version = 2;
    assert!(library.validate().is_err());
    for date in ["2026-2-01", "2026-02-29", "0000-01-01"] {
        assert!(Fields {
            title: "a".into(),
            due: Some(date.into()),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
    for time in ["1:00", "24:00", "09:01:00"] {
        assert!(Reminder {
            date: "2026-11-26".into(),
            time: time.into()
        }
        .local_timestamp()
        .is_err());
    }
    assert!(Fields {
        title: "a\nb".into(),
        ..Default::default()
    }
    .validate()
    .is_err());
    assert!(Fields {
        title: "a".into(),
        notes: "x".repeat(MAX_NOTES_BYTES + 1),
        ..Default::default()
    }
    .validate()
    .is_err());
}
