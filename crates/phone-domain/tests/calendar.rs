use phone_domain::*;

fn event(date: &str, repeat: CalendarRepeat) -> CalendarEvent {
    CalendarEvent {
        id: 17,
        title: "Planning".into(),
        date: date.into(),
        time: "09:30".into(),
        duration_minutes: 60,
        repeat,
        ..Default::default()
    }
}
fn dates(event: CalendarEvent, from: &str, to: &str) -> Vec<String> {
    event_occurrences(
        &[event],
        calendar_day(from).unwrap(),
        calendar_day(to).unwrap(),
    )
    .into_iter()
    .map(|item| item.date)
    .collect()
}

#[test]
fn old_appointments_default_to_nonrepeating_and_dates_are_canonical() {
    let old = r#"{"id":17,"title":"Planning","date":"2026-11-23","time":"09:30","duration_minutes":60,"location":"","details":""}"#;
    let parsed: CalendarEvent = serde_json::from_str(old).unwrap();
    assert_eq!(parsed.repeat, CalendarRepeat::Never);
    assert!(!parsed.all_day);
    assert!(parsed.reminder_minutes.is_none());
    validate_calendar_event(&parsed).unwrap();
    for invalid in ["2026-2-03", "2026-02-30", "0000-01-01", "２０２６-11-23"] {
        assert!(calendar_day(invalid).is_none());
    }
    for invalid in ["9:30", "24:00", "10:60", "ab:00"] {
        assert!(calendar_minute(invalid).is_none());
    }
}

#[test]
fn weekday_weekly_and_biweekly_rules_preserve_the_original_schedule() {
    assert_eq!(
        dates(
            event("2026-11-20", CalendarRepeat::Weekdays),
            "2026-11-20",
            "2026-11-25"
        ),
        ["2026-11-20", "2026-11-23", "2026-11-24"]
    );
    assert_eq!(
        dates(
            event("2026-11-23", CalendarRepeat::Weekly),
            "2026-12-01",
            "2026-12-16"
        ),
        ["2026-12-07", "2026-12-14"]
    );
    assert_eq!(
        dates(
            event("2026-11-23", CalendarRepeat::Biweekly),
            "2026-11-23",
            "2026-12-22"
        ),
        ["2026-11-23", "2026-12-07", "2026-12-21"]
    );
}

#[test]
fn monthly_and_yearly_rules_skip_impossible_dates_including_centuries() {
    assert_eq!(
        dates(
            event("2026-01-31", CalendarRepeat::Monthly),
            "2026-01-01",
            "2026-05-01"
        ),
        ["2026-01-31", "2026-03-31"]
    );
    let leap = event("2096-02-29", CalendarRepeat::Yearly);
    let next =
        next_calendar_occurrence(&[leap], calendar_day("2097-01-01").unwrap() * 86_400).unwrap();
    assert_eq!(next.date, "2104-02-29");
    let ancient = event("0001-01-01", CalendarRepeat::Daily);
    assert_eq!(
        dates(ancient, "9999-12-29", "9999-12-31"),
        ["9999-12-29", "9999-12-30"]
    );
}

#[test]
fn recurrence_end_dates_exceptions_and_queries_do_not_mutate_events() {
    let mut series = event("2026-11-23", CalendarRepeat::Daily);
    series.repeat_until = Some("2026-11-26".into());
    series.excluded_dates = vec!["2026-11-24".into()];
    let before = series.clone();
    validate_calendar_event(&series).unwrap();
    assert_eq!(
        dates(series.clone(), "2026-11-23", "2026-11-30"),
        ["2026-11-23", "2026-11-25", "2026-11-26"]
    );
    assert_eq!(series, before);
    series.excluded_dates.push("2026-11-24".into());
    assert!(validate_calendar_event(&series).is_err());
    series.excluded_dates = vec!["2026-11-27".into()];
    assert!(validate_calendar_event(&series).is_err());
}

#[test]
fn overnight_and_all_day_events_intersect_the_visible_day_and_finished_events_are_skipped() {
    let mut overnight = event("2026-11-23", CalendarRepeat::Never);
    overnight.time = "23:30".into();
    overnight.duration_minutes = 120;
    let mut all_day = event("2026-11-24", CalendarRepeat::Never);
    all_day.id = 18;
    all_day.all_day = true;
    all_day.duration_minutes = 1440;
    let day = calendar_day("2026-11-24").unwrap();
    let items = event_occurrences(&[overnight.clone(), all_day.clone()], day, day + 1);
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].start, day * 86_400);
    assert_eq!(
        next_calendar_occurrence(&[overnight.clone(), all_day.clone()], day * 86_400 + 3600)
            .unwrap()
            .event_id,
        17
    );
    assert_eq!(
        next_calendar_occurrence(&[overnight, all_day], day * 86_400 + 5400)
            .unwrap()
            .event_id,
        18
    );
}

#[test]
fn expansion_is_bounded_and_invalid_backups_reject_calendar_data() {
    let start = calendar_day("2026-01-01").unwrap();
    assert_eq!(
        event_occurrences(
            &[event("2026-01-01", CalendarRepeat::Daily)],
            start,
            start + 10_000
        )
        .len(),
        366
    );
    assert!(event_occurrences(&[], i64::MAX - 1, i64::MAX).is_empty());
    assert!(next_calendar_occurrence(&[], i64::MAX).is_none());
    let series = event("0001-01-01", CalendarRepeat::Daily);
    for day in [
        i64::MIN,
        i64::MAX,
        days_from_date(0, 12, 31),
        days_from_date(10_000, 1, 1),
    ] {
        assert!(!event_occurs_on(&series, day));
    }
    assert!(event_occurs_on(&series, days_from_date(1, 1, 1)));
    assert!(event_occurs_on(&series, days_from_date(9999, 12, 31)));
    let mut data = PhoneData::default();
    data.events = vec![event("2026-02-30", CalendarRepeat::Never)];
    assert!(data.normalize().is_err());
    let mut valid = event("2026-11-23", CalendarRepeat::Daily);
    valid.all_day = true;
    valid.duration_minutes = 60;
    assert!(validate_calendar_event(&valid).is_err());
    valid.all_day = false;
    valid.reminder_minutes = Some(10_081);
    assert!(validate_calendar_event(&valid).is_err());
}
