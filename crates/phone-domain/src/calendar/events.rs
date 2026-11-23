use crate::{date_parts, date_string, days_from_date, days_in_month, CalendarEvent, Id};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_CALENDAR_EVENTS: usize = 4_096;
pub const MAX_CALENDAR_EXCEPTIONS: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarRepeat {
    #[default]
    Never,
    Daily,
    Weekdays,
    Weekly,
    Biweekly,
    Monthly,
    Yearly,
}
impl CalendarRepeat {
    pub const ALL: [Self; 7] = [
        Self::Never,
        Self::Daily,
        Self::Weekdays,
        Self::Weekly,
        Self::Biweekly,
        Self::Monthly,
        Self::Yearly,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "only once",
            Self::Daily => "every day",
            Self::Weekdays => "every weekday",
            Self::Weekly => "every week",
            Self::Biweekly => "every two weeks",
            Self::Monthly => "every month",
            Self::Yearly => "every year",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarStatus {
    #[default]
    Free,
    Tentative,
    Busy,
    OutOfOffice,
}
impl CalendarStatus {
    pub const ALL: [Self; 4] = [Self::Free, Self::Tentative, Self::Busy, Self::OutOfOffice];
    pub fn label(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Tentative => "tentative",
            Self::Busy => "busy",
            Self::OutOfOffice => "out of office",
        }
    }
}

#[derive(Clone, Debug)]
pub enum CalendarAction {
    /// `event.id` identifies the series; the replacement becomes a standalone event.
    SaveOccurrence {
        event: CalendarEvent,
        occurrence_date: String,
    },
    DeleteOccurrence {
        id: Id,
        date: String,
    },
}

/// A local wall-clock occurrence; recurrence is expanded only for the requested range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarOccurrence {
    pub event_id: Id,
    pub date: String,
    pub start: i64,
    pub end: i64,
}

/// Parse the canonical storage format before any recurrence arithmetic.
pub fn calendar_day(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return None;
    }
    let year: i32 = value[..4].parse().ok()?;
    let month: u32 = value[5..7].parse().ok()?;
    let day: u32 = value[8..].parse().ok()?;
    ((1..=9999).contains(&year) && day > 0 && day <= days_in_month(year, month))
        .then(|| days_from_date(year, month, day))
}

pub fn calendar_date(day: i64) -> String {
    let (year, month, day) = date_parts(day.saturating_mul(86_400));
    date_string(year, month, day)
}

pub fn calendar_minute(value: &str) -> Option<u32> {
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 2 || b.is_ascii_digit())
    {
        return None;
    }
    let hour: u32 = value[..2].parse().ok()?;
    let minute: u32 = value[3..].parse().ok()?;
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

pub fn validate_calendar_event(event: &CalendarEvent) -> Result<(), String> {
    if event.title.trim().is_empty() || event.title.chars().count() > 256 {
        return Err("Give your appointment a subject of up to 256 characters.".into());
    }
    if event.location.len() > 4_096 || event.details.len() > 65_536 {
        return Err("This appointment's location or notes are too long.".into());
    }
    let start = calendar_day(&event.date).ok_or("Enter a valid date in YYYY-MM-DD format.")?;
    calendar_minute(&event.time).ok_or("Enter a valid time in HH:MM format.")?;
    if !(1..=10_080).contains(&event.duration_minutes) {
        return Err("Choose a duration between 1 minute and 7 days.".into());
    }
    if event.all_day && event.duration_minutes % 1_440 != 0 {
        return Err("An all-day appointment must last a whole number of days.".into());
    }
    if let Some(until) = &event.repeat_until {
        if calendar_day(until).is_none_or(|until| until < start) {
            return Err("The repeat end date must be on or after the first appointment.".into());
        }
    }
    if event
        .reminder_minutes
        .is_some_and(|minutes| minutes > 10_080)
    {
        return Err("Choose a reminder no more than one week before the appointment.".into());
    }
    if event.excluded_dates.len() > MAX_CALENDAR_EXCEPTIONS {
        return Err("This series has reached its limit of 512 exceptions.".into());
    }
    let mut unique = HashSet::new();
    for date in &event.excluded_dates {
        let day =
            calendar_day(date).ok_or("A recurring appointment has an invalid exception date.")?;
        if event.repeat == CalendarRepeat::Never
            || !unique.insert(date)
            || !super::recurrence::scheduled_on(event, day)
        {
            return Err(
                "A recurring appointment contains an invalid or duplicate exception.".into(),
            );
        }
    }
    Ok(())
}

pub fn validate_calendar_events(events: &[CalendarEvent]) -> Result<(), String> {
    if events.len() > MAX_CALENDAR_EVENTS {
        return Err("The phone calendar can hold up to 4,096 appointments and series.".into());
    }
    for event in events {
        validate_calendar_event(event)?;
    }
    Ok(())
}
