//! Atomic exceptions for a recurring appointment; series edits use SaveEvent.
use phone_domain::*;

pub(crate) fn mutate(data: &mut PhoneData, action: CalendarAction) -> Result<bool, String> {
    let (id, date) = match &action {
        CalendarAction::SaveOccurrence {
            event,
            occurrence_date,
        } => (event.id, occurrence_date),
        CalendarAction::DeleteOccurrence { id, date } => (*id, date),
    };
    let index = data
        .events
        .iter()
        .position(|event| event.id == id)
        .ok_or("This appointment no longer exists.")?;
    let day = calendar_day(date).ok_or("Choose a valid occurrence date.")?;
    let mut series = data.events[index].clone();
    if series.repeat == CalendarRepeat::Never || !event_occurs_on(&series, day) {
        return Err("This occurrence no longer belongs to the repeating appointment.".into());
    }
    series.excluded_dates.push(date.clone());
    series.excluded_dates.sort();
    validate_calendar_event(&series)?;
    match action {
        CalendarAction::DeleteOccurrence { .. } => data.events[index] = series,
        CalendarAction::SaveOccurrence { mut event, .. } => {
            if data.events.len() >= MAX_CALENDAR_EVENTS {
                return Err(
                    "The phone calendar can hold up to 4,096 appointments and series.".into(),
                );
            }
            event.repeat = CalendarRepeat::Never;
            event.repeat_until = None;
            event.excluded_dates.clear();
            validate_calendar_event(&event)?;
            if data.next_id == Id::MAX {
                return Err("No appointment identifiers remain.".into());
            }
            event.id = data.allocate_id();
            data.events[index] = series;
            data.events.push(event);
        }
    }
    Ok(true)
}
