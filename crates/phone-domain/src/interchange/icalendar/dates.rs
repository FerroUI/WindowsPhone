use super::super::text::Property;
use crate::{calendar_day, calendar_minute};

pub(super) struct DateValue {
    pub date: String,
    pub time: String,
    pub all_day: bool,
}
impl DateValue {
    pub fn minute(&self) -> i64 {
        // Both fields are validated by date_value before a DateValue is made.
        calendar_day(&self.date).unwrap() * 1_440 + i64::from(calendar_minute(&self.time).unwrap())
    }
}

pub(super) fn date_value(value: &Property) -> Result<DateValue, String> {
    let all_day = match value
        .params
        .get("VALUE")
        .map(|v| v.to_ascii_uppercase())
        .as_deref()
    {
        Some("DATE") => true,
        Some("DATE-TIME") | None => false,
        _ => return Err("Unsupported calendar date value type.".into()),
    };
    let raw = value.value.as_str();
    if raw.ends_with('Z') || value.params.contains_key("TZID") {
        return Err("This phone calendar stores local times. UTC and time-zone appointments require a calendar provider.".into());
    }
    if raw.len() != if all_day { 8 } else { 15 }
        || !raw.as_bytes()[..8].iter().all(u8::is_ascii_digit)
    {
        return Err(
            "Use calendar dates as YYYYMMDD and local date-times as YYYYMMDDTHHMMSS.".into(),
        );
    }
    let date = format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..8]);
    if calendar_day(&date).is_none() {
        return Err("The calendar contains an invalid date.".into());
    }
    let time = if all_day {
        "00:00".into()
    } else {
        if raw.as_bytes()[8] != b'T'
            || !raw.as_bytes()[9..].iter().all(u8::is_ascii_digit)
            || &raw[13..] != "00"
        {
            return Err("Appointment start and end times must use whole minutes.".into());
        }
        format!("{}:{}", &raw[9..11], &raw[11..13])
    };
    if calendar_minute(&time).is_none() {
        return Err("The calendar contains an invalid time.".into());
    }
    Ok(DateValue {
        date,
        time,
        all_day,
    })
}

pub(super) fn validate_stamp(value: &str) -> Result<(), String> {
    if value.len() != 16
        || !value.ends_with('Z')
        || value.as_bytes()[8] != b'T'
        || value
            .bytes()
            .enumerate()
            .any(|(i, b)| i != 8 && i != 15 && !b.is_ascii_digit())
    {
        return Err("Calendar DTSTAMP must be a UTC timestamp.".into());
    }
    let date = format!("{}-{}-{}", &value[..4], &value[4..6], &value[6..8]);
    let time = format!("{}:{}", &value[9..11], &value[11..13]);
    if calendar_day(&date).is_none()
        || calendar_minute(&time).is_none()
        || value[13..15].parse::<u8>().unwrap_or(255) > 60
    {
        return Err("The calendar contains an invalid timestamp.".into());
    }
    Ok(())
}

pub(super) fn duration(value: &str, negative: bool) -> Result<u32, String> {
    let value = if negative {
        value
            .strip_prefix('-')
            .ok_or("A reminder must occur at or before the appointment; use a negative duration.")?
    } else {
        value.strip_prefix('+').unwrap_or(value)
    };
    let value = value
        .strip_prefix('P')
        .ok_or("Invalid calendar duration.")?;
    let mut number = String::new();
    let mut time = false;
    let mut seen = false;
    let mut rank = 0;
    let mut total = 0u32;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            number.push(ch);
            if number.len() > 6 {
                return Err("Calendar duration is too large.".into());
            }
            continue;
        }
        if ch == 'T' && !time && number.is_empty() {
            time = true;
            continue;
        }
        let (order, multiplier) = match (time, ch) {
            (false, 'W') => (1, 10_080),
            (false, 'D') => (2, 1_440),
            (true, 'H') => (3, 60),
            (true, 'M') => (4, 1),
            (true, 'S') => (5, 0),
            _ => return Err("Unsupported calendar duration units.".into()),
        };
        if order <= rank || number.is_empty() || (rank == 1 && order != 1) {
            return Err("Invalid calendar duration order.".into());
        }
        let quantity = number
            .parse::<u32>()
            .map_err(|_| "Invalid calendar duration.")?;
        if ch == 'S' && quantity != 0 {
            return Err("Appointments and reminders use whole minutes.".into());
        }
        total = total
            .checked_add(
                quantity
                    .checked_mul(multiplier)
                    .ok_or("Calendar duration is too large.")?,
            )
            .ok_or("Calendar duration is too large.")?;
        if total > 10_080 {
            return Err("Appointments and reminder leads can span up to one week.".into());
        }
        number.clear();
        rank = order;
        seen = true;
    }
    if !seen || !number.is_empty() || value.ends_with('T') {
        return Err("Incomplete calendar duration.".into());
    }
    Ok(total)
}
