use super::super::text::*;
use super::{availability, dates, recurrence, reminder::reminder};
use crate::{CalendarEvent, CalendarStatus, validate_calendar_event};
use std::collections::BTreeMap;

pub(super) fn appointment(
    values: &[Property],
    alarm: Option<Vec<Property>>,
) -> Result<(CalendarEvent, String), String> {
    let mut unique = BTreeMap::new();
    let mut exceptions = Vec::new();
    for value in values {
        if value.name == "EXDATE" {
            exceptions.push(value);
            continue;
        }
        if unique.insert(value.name.as_str(), value).is_some() {
            return Err(format!("Duplicate appointment property {}.", value.name));
        }
        match value.name.as_str() {
            "UID"
            | "DTSTAMP"
            | "CREATED"
            | "LAST-MODIFIED"
            | "SEQUENCE"
            | "SUMMARY"
            | "DESCRIPTION"
            | "LOCATION"
            | "RRULE"
            | "TRANSP"
            | "STATUS"
            | "X-MICROSOFT-CDO-BUSYSTATUS" => allowed_params(value, &[])?,
            "DTSTART" | "DTEND" => allowed_params(value, &["VALUE"])?,
            "DURATION" => allowed_params(value, &[])?,
            _ => {
                return Err(format!(
                    "Appointment field {} cannot be preserved. Meetings, time zones and advanced recurrence need a calendar provider.",
                    value.name
                ));
            }
        }
    }
    let required = |name: &str| {
        unique
            .get(name)
            .copied()
            .ok_or_else(|| format!("The appointment needs {name}."))
    };
    let uid = required("UID")?.value.clone();
    if uid.is_empty() || uid.len() > 4_096 {
        return Err("The appointment has an invalid UID.".into());
    }
    if let Some(stamp) = unique.get("DTSTAMP") {
        dates::validate_stamp(&stamp.value)?;
    }
    let start = dates::date_value(required("DTSTART")?)?;
    let mut item = CalendarEvent {
        title: unescape(&required("SUMMARY")?.value)?,
        date: start.date.clone(),
        time: start.time.clone(),
        all_day: start.all_day,
        status: CalendarStatus::Busy,
        ..Default::default()
    };
    if let Some(value) = unique.get("LOCATION") {
        item.location = unescape(&value.value)?;
    }
    if let Some(value) = unique.get("DESCRIPTION") {
        item.details = unescape(&value.value)?;
    }
    item.duration_minutes = match (unique.get("DTEND"), unique.get("DURATION")) {
        (Some(_), Some(_)) => {
            return Err("An appointment cannot contain both DTEND and DURATION.".into());
        }
        (Some(end), None) => {
            let end = dates::date_value(end)?;
            if end.all_day != start.all_day {
                return Err("The appointment's start and end value types must match.".into());
            }
            u32::try_from(end.minute() - start.minute())
                .map_err(|_| "The appointment ends before it begins.")?
        }
        (None, Some(duration)) => dates::duration(&duration.value, false)?,
        (None, None) if start.all_day => 1_440,
        _ => return Err("A timed appointment needs an end or duration.".into()),
    };
    if let Some(rule) = unique.get("RRULE") {
        recurrence::apply(&mut item, &rule.value)?;
    }
    for excluded in exceptions {
        allowed_params(excluded, &["VALUE"])?;
        for value in excluded.value.split(',') {
            let date = dates::date_value(&Property {
                name: "EXDATE".into(),
                params: excluded.params.clone(),
                value: value.into(),
            })?;
            if date.all_day != item.all_day || (!item.all_day && date.time != item.time) {
                return Err(
                    "Exception dates must match the series value type and start time.".into(),
                );
            }
            item.excluded_dates.push(date.date);
        }
    }
    item.status = availability::read(&unique)?;
    if let Some(alarm) = alarm {
        item.reminder_minutes = Some(reminder(&alarm, &item.title)?);
    }
    validate_calendar_event(&item)?;
    Ok((item, uid))
}
