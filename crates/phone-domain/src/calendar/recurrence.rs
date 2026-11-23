use super::{CalendarOccurrence, CalendarRepeat, calendar_date, calendar_day, calendar_minute};
use crate::{CalendarEvent, date_parts, days_from_date, days_in_month};

pub const MAX_OCCURRENCE_WINDOW_DAYS: i64 = 366;
pub const MAX_CALENDAR_OCCURRENCES: usize = 4_096;

pub(crate) fn scheduled_on(event: &CalendarEvent, day: i64) -> bool {
    // Callers such as foreground reminder clocks may receive external times.
    // Guard before converting days to seconds, which could otherwise overflow.
    if !(days_from_date(1, 1, 1)..=days_from_date(9999, 12, 31)).contains(&day) {
        return false;
    }
    let Some(first) = calendar_day(&event.date) else {
        return false;
    };
    if day < first
        || event
            .repeat_until
            .as_deref()
            .and_then(calendar_day)
            .is_some_and(|last| day > last)
    {
        return false;
    }
    let (year, month, date) = date_parts(day * 86_400);
    let (first_year, first_month, first_date) = date_parts(first * 86_400);
    match event.repeat {
        CalendarRepeat::Never => day == first,
        CalendarRepeat::Daily => true,
        CalendarRepeat::Weekdays => (day + 3).rem_euclid(7) < 5,
        CalendarRepeat::Weekly => (day - first) % 7 == 0,
        CalendarRepeat::Biweekly => (day - first) % 14 == 0,
        CalendarRepeat::Monthly => date == first_date && (year, month) >= (first_year, first_month),
        CalendarRepeat::Yearly => month == first_month && date == first_date,
    }
}

fn next_scheduled(event: &CalendarEvent, day: i64) -> Option<i64> {
    let first = calendar_day(&event.date)?;
    let day = day.max(first);
    let candidate = match event.repeat {
        CalendarRepeat::Never => (day <= first).then_some(first)?,
        CalendarRepeat::Daily => day,
        CalendarRepeat::Weekdays => {
            day + match (day + 3).rem_euclid(7) {
                5 => 2,
                6 => 1,
                _ => 0,
            }
        }
        CalendarRepeat::Weekly | CalendarRepeat::Biweekly => {
            let interval = if event.repeat == CalendarRepeat::Weekly {
                7
            } else {
                14
            };
            first + (day - first + interval - 1) / interval * interval
        }
        CalendarRepeat::Monthly => {
            let (_, _, date) = date_parts(first * 86_400);
            let (year, month, _) = date_parts(day * 86_400);
            let mut index = year * 12 + month as i32 - 1;
            // A skipped 29th/30th/31st is never clamped into a different date.
            let mut found = None;
            for _ in 0..3 {
                let year = index.div_euclid(12);
                let month = index.rem_euclid(12) as u32 + 1;
                if (1..=9999).contains(&year) && date <= days_in_month(year, month) {
                    let candidate = days_from_date(year, month, date);
                    if candidate >= day {
                        found = Some(candidate);
                        break;
                    }
                }
                index += 1;
            }
            found?
        }
        CalendarRepeat::Yearly => {
            let (_, month, date) = date_parts(first * 86_400);
            let (year, _, _) = date_parts(day * 86_400);
            let mut found = None;
            // Eight years covers leap-day series across a Gregorian century gap.
            for year in year..=(year + 8).min(9999) {
                if date <= days_in_month(year, month) {
                    let candidate = days_from_date(year, month, date);
                    if candidate >= day {
                        found = Some(candidate);
                        break;
                    }
                }
            }
            found?
        }
    };
    let (year, _, _) = date_parts(candidate * 86_400);
    if !(1..=9999).contains(&year)
        || event
            .repeat_until
            .as_deref()
            .and_then(calendar_day)
            .is_some_and(|last| candidate > last)
    {
        None
    } else {
        Some(candidate)
    }
}

pub fn event_occurs_on(event: &CalendarEvent, day: i64) -> bool {
    scheduled_on(event, day)
        && !event
            .excluded_dates
            .iter()
            .any(|date| calendar_day(date) == Some(day))
}

fn occurrence(event: &CalendarEvent, day: i64) -> Option<CalendarOccurrence> {
    let minute = if event.all_day {
        0
    } else {
        calendar_minute(&event.time)? as i64
    };
    let start = day * 86_400 + minute * 60;
    Some(CalendarOccurrence {
        event_id: event.id,
        date: calendar_date(day),
        start,
        end: start + i64::from(event.duration_minutes) * 60,
    })
}

/// Query occurrences intersecting [from_day, to_day). At most one year and 4,096
/// results are materialized; old series are reached by arithmetic, never replayed.
pub fn event_occurrences(
    events: &[CalendarEvent],
    from_day: i64,
    to_day: i64,
) -> Vec<CalendarOccurrence> {
    if to_day <= from_day || from_day > days_from_date(9999, 12, 31) {
        return Vec::new();
    }
    let from_day = from_day.max(days_from_date(1, 1, 1));
    let to_day = to_day
        .min(from_day.saturating_add(MAX_OCCURRENCE_WINDOW_DAYS))
        .min(days_from_date(9999, 12, 31) + 1);
    let mut result = Vec::new();
    for event in events {
        if event.duration_minutes == 0 || event.duration_minutes > 10_080 {
            continue;
        }
        let mut cursor = from_day.saturating_sub(7);
        while let Some(day) = next_scheduled(event, cursor) {
            if day >= to_day {
                break;
            }
            if event_occurs_on(event, day) {
                if let Some(item) =
                    occurrence(event, day).filter(|item| item.end > from_day * 86_400)
                {
                    result.push(item);
                    if result.len() >= MAX_CALENDAR_OCCURRENCES {
                        break;
                    }
                }
            }
            cursor = day + 1;
        }
        if result.len() >= MAX_CALENDAR_OCCURRENCES {
            break;
        }
    }
    result.sort_by(|a, b| (a.start, a.end, a.event_id).cmp(&(b.start, b.end, b.event_id)));
    result
}

/// Find the earliest ongoing or upcoming occurrence, including long-lived and
/// leap-day series. Exceptions add bounded work (at most 512 skipped dates).
pub fn next_calendar_occurrence(events: &[CalendarEvent], now: i64) -> Option<CalendarOccurrence> {
    let today = now.div_euclid(86_400);
    if !(days_from_date(1, 1, 1)..=days_from_date(9999, 12, 31)).contains(&today) {
        return None;
    }
    events
        .iter()
        .filter_map(|event| {
            let mut cursor = today.saturating_sub(7);
            for _ in 0..520 {
                let day = next_scheduled(event, cursor)?;
                if event_occurs_on(event, day) {
                    if let Some(item) = occurrence(event, day).filter(|item| item.end > now) {
                        return Some(item);
                    }
                }
                cursor = day + 1;
            }
            None
        })
        .min_by_key(|item| (item.start, item.end, item.event_id))
}
