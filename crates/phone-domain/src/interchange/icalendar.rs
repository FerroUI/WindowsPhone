mod appointment;
mod availability;
mod dates;
mod export;
mod recurrence;
mod reminder;

use super::{MAX_INTERCHANGE_ITEMS, text::*};
use crate::CalendarEvent;
use appointment::appointment;
pub use export::export_icalendar;
use std::collections::BTreeSet;

/// Local wall-clock and all-day VEVENT imports. Time zones, meetings, tasks,
/// detached recurrence overrides and unsupported rules are rejected explicitly.
pub fn import_icalendar(bytes: &[u8]) -> Result<Vec<CalendarEvent>, String> {
    let mut result = Vec::new();
    let mut calendar = false;
    let mut ended = false;
    let mut version = false;
    let mut product = false;
    let mut event: Option<Vec<Property>> = None;
    let mut alarm: Option<Vec<Property>> = None;
    let mut event_alarm = None;
    let mut identities = BTreeSet::new();
    for line in lines(bytes)? {
        let value = property(&line)?;
        match (value.name.as_str(), value.value.as_str()) {
            ("BEGIN", "VCALENDAR") if !calendar && !ended => {
                calendar = true;
                continue;
            }
            ("END", "VCALENDAR") if calendar && event.is_none() => {
                calendar = false;
                ended = true;
                continue;
            }
            ("BEGIN", "VEVENT") if calendar && event.is_none() => {
                event = Some(Vec::new());
                continue;
            }
            ("BEGIN", "VALARM") if event.is_some() && alarm.is_none() && event_alarm.is_none() => {
                alarm = Some(Vec::new());
                continue;
            }
            ("END", "VALARM") if alarm.is_some() => {
                event_alarm = alarm.take();
                continue;
            }
            ("END", "VEVENT") if event.is_some() && alarm.is_none() => {
                if result.len() >= MAX_INTERCHANGE_ITEMS {
                    return Err("Import up to 512 appointments at a time.".into());
                }
                let (item, uid) = appointment(&event.take().unwrap(), event_alarm.take())?;
                if !identities.insert(uid) {
                    return Err("Repeated calendar identities or detached occurrences cannot be imported as separate series.".into());
                }
                result.push(item);
                continue;
            }
            ("BEGIN" | "END", _) => {
                return Err(format!(
                    "Calendar component {} is incomplete or unsupported.",
                    value.value
                ));
            }
            _ => {}
        }
        if let Some(alarm) = alarm.as_mut() {
            alarm.push(value);
        } else if let Some(event) = event.as_mut() {
            event.push(value);
        } else if calendar {
            allowed_params(&value, &[])?;
            match value.name.as_str() {
                "VERSION" if value.value == "2.0" && !version => version = true,
                "PRODID" if !product && !value.value.is_empty() => product = true,
                "CALSCALE" if value.value.eq_ignore_ascii_case("GREGORIAN") => {}
                "METHOD" if value.value.eq_ignore_ascii_case("PUBLISH") => {}
                _ => {
                    return Err(format!(
                        "Calendar property {} is unsupported or duplicated.",
                        value.name
                    ));
                }
            }
        } else {
            return Err("Choose an iCalendar (.ics) appointment file.".into());
        }
    }
    if calendar
        || event.is_some()
        || alarm.is_some()
        || !ended
        || !version
        || !product
        || result.is_empty()
    {
        return Err("The calendar file is empty or incomplete.".into());
    }
    Ok(result)
}
