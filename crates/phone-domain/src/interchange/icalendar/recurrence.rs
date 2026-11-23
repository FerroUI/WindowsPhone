use super::{super::text::Property, dates};
use crate::{CalendarEvent, CalendarRepeat, calendar_date, calendar_day, calendar_minute};
use std::collections::BTreeMap;

pub(super) fn apply(event: &mut CalendarEvent, value: &str) -> Result<(), String> {
    let mut parts = BTreeMap::new();
    for item in value.split(';') {
        let (key, value) = item
            .split_once('=')
            .ok_or("Invalid calendar recurrence rule.")?;
        let key = key.to_ascii_uppercase();
        if !["FREQ", "INTERVAL", "BYDAY", "UNTIL", "WKST"].contains(&key.as_str())
            || parts.insert(key, value.to_ascii_uppercase()).is_some()
        {
            return Err("This recurrence rule cannot be preserved. Use daily, weekdays, weekly, biweekly, monthly or yearly recurrence with an optional end date.".into());
        }
    }
    let frequency = parts
        .get("FREQ")
        .ok_or("A recurrence rule needs a frequency.")?
        .as_str();
    let interval = parts.get("INTERVAL").map(String::as_str).unwrap_or("1");
    let days = parts
        .get("BYDAY")
        .map(|days| {
            let mut v = days.split(',').collect::<Vec<_>>();
            v.sort();
            v
        })
        .unwrap_or_default();
    event.repeat = match (frequency, interval, days.as_slice()) {
        ("DAILY", "1", []) => CalendarRepeat::Daily,
        ("WEEKLY", "1", ["FR", "MO", "TH", "TU", "WE"]) => CalendarRepeat::Weekdays,
        ("WEEKLY", "1", []) => CalendarRepeat::Weekly,
        ("WEEKLY", "2", []) => CalendarRepeat::Biweekly,
        ("MONTHLY", "1", []) => CalendarRepeat::Monthly,
        ("YEARLY", "1", []) => CalendarRepeat::Yearly,
        _ => return Err(
            "This recurrence interval or day selection cannot be preserved by the phone calendar."
                .into(),
        ),
    };
    if event.repeat == CalendarRepeat::Weekdays
        && (calendar_day(&event.date).unwrap() + 3).rem_euclid(7) >= 5
    {
        return Err(
            "A weekday series that starts on a weekend cannot be preserved by this phone.".into(),
        );
    }
    if let Some(day) = parts.get("WKST") {
        if !["MO", "TU", "WE", "TH", "FR", "SA", "SU"].contains(&day.as_str()) {
            return Err("Invalid recurrence week start.".into());
        }
        // The accepted weekly patterns select the DTSTART weekday or every
        // weekday, so week grouping cannot alter their occurrence set.
    }
    if let Some(until) = parts.get("UNTIL") {
        let mut params = BTreeMap::new();
        if event.all_day {
            params.insert("VALUE".into(), "DATE".into());
        }
        let last = dates::date_value(&Property {
            name: "UNTIL".into(),
            params,
            value: until.clone(),
        })?;
        let mut day = calendar_day(&last.date).unwrap();
        if !event.all_day && calendar_minute(&last.time) < calendar_minute(&event.time) {
            day -= 1;
        }
        event.repeat_until = Some(calendar_date(day));
    }
    Ok(())
}
