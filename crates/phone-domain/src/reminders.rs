//! Foreground calendar reminders: one point query per series and minute.
use crate::{
    CalendarEvent, CalendarOccurrence, Id, MAX_CALENDAR_EVENTS, calendar_date, calendar_day,
    calendar_minute, event_occurs_on,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarReminder {
    pub occurrence: CalendarOccurrence,
    pub title: String,
    pub location: String,
}

#[derive(Default, Debug)]
pub struct ReminderSchedule {
    last_minute: Option<i64>,
    fired: BTreeMap<Id, i64>,
    snoozed: BTreeMap<(Id, i64), (CalendarReminder, i64)>,
}

pub fn reminder_is_current(reminder: &CalendarReminder, events: &[CalendarEvent]) -> bool {
    events
        .iter()
        .find(|event| event.id == reminder.occurrence.event_id)
        .is_some_and(|event| reminder_matches_event(reminder, event))
}

fn reminder_matches_event(reminder: &CalendarReminder, event: &CalendarEvent) -> bool {
    let Some(day) = calendar_day(&reminder.occurrence.date) else {
        return false;
    };
    let minute = if event.all_day {
        Some(0)
    } else {
        calendar_minute(&event.time)
    };
    event.reminder_minutes.is_some()
        && event_occurs_on(event, day)
        && minute.is_some_and(|minute| {
            day * 86_400 + i64::from(minute) * 60 == reminder.occurrence.start
        })
}

impl ReminderSchedule {
    pub fn poll(
        &mut self,
        events: &[CalendarEvent],
        now: i64,
        offset: i32,
    ) -> Vec<CalendarReminder> {
        let local = now.saturating_add(i64::from(offset));
        let first_day = crate::days_from_date(1, 1, 1);
        let last_day = crate::days_from_date(9999, 12, 31);
        if !(first_day..=last_day).contains(&local.div_euclid(86_400)) {
            return Vec::new();
        }
        let minute = local.div_euclid(60);
        let changed = self.last_minute.replace(minute) != Some(minute);
        if !changed && self.snoozed.is_empty() {
            return Vec::new();
        }
        let by_id = events
            .iter()
            .map(|event| (event.id, event))
            .collect::<HashMap<_, _>>();
        self.fired.retain(|id, _| by_id.contains_key(id));
        self.snoozed.retain(|_, (reminder, _)| {
            by_id
                .get(&reminder.occurrence.event_id)
                .is_some_and(|event| reminder_matches_event(reminder, event))
        });
        let ready = self
            .snoozed
            .iter()
            .filter_map(|(key, (_, due))| (now >= *due).then_some(*key))
            .collect::<Vec<_>>();
        let mut due = ready
            .into_iter()
            .filter_map(|key| {
                let (mut item, _) = self.snoozed.remove(&key)?;
                let event = by_id.get(&item.occurrence.event_id)?;
                // A snooze retains its due time, not a stale copy of an edited
                // appointment's subject, location or duration.
                item.title = event.title.clone();
                item.location = event.location.clone();
                item.occurrence.end = item
                    .occurrence
                    .start
                    .saturating_add(i64::from(event.duration_minutes) * 60);
                Some(item)
            })
            .collect::<Vec<_>>();
        if !changed {
            return due;
        }
        for event in events.iter().take(MAX_CALENDAR_EVENTS) {
            let Some(lead) = event.reminder_minutes.filter(|lead| *lead <= 10_080) else {
                continue;
            };
            let start_minute = minute.saturating_add(i64::from(lead));
            let day = start_minute.div_euclid(1_440);
            if !(first_day..=last_day).contains(&day) {
                continue;
            }
            let clock = if event.all_day {
                Some(0)
            } else {
                calendar_minute(&event.time)
            };
            if clock != Some(start_minute.rem_euclid(1_440) as u32) || !event_occurs_on(event, day)
            {
                continue;
            }
            let start = start_minute.saturating_mul(60);
            if self
                .fired
                .get(&event.id)
                .is_some_and(|previous| *previous >= start)
            {
                continue;
            }
            self.fired.insert(event.id, start);
            due.push(CalendarReminder {
                occurrence: CalendarOccurrence {
                    event_id: event.id,
                    date: calendar_date(day),
                    start,
                    end: start.saturating_add(i64::from(event.duration_minutes) * 60),
                },
                title: event.title.clone(),
                location: event.location.clone(),
            });
        }
        due
    }

    /// Recheck changed appointments this minute without replaying delivered
    /// occurrences or discarding an explicitly snoozed reminder.
    pub fn invalidate(&mut self) {
        self.last_minute = None;
    }

    pub fn snooze(&mut self, reminder: CalendarReminder, now: i64, minutes: u8) {
        let key = (reminder.occurrence.event_id, reminder.occurrence.start);
        if self.snoozed.len() >= MAX_CALENDAR_EVENTS && !self.snoozed.contains_key(&key) {
            return;
        }
        self.snoozed.insert(
            key,
            (
                reminder,
                now.saturating_add(i64::from(minutes.clamp(1, 60)) * 60),
            ),
        );
    }
    pub fn cancel(&mut self, item: &CalendarReminder) {
        self.snoozed
            .remove(&(item.occurrence.event_id, item.occurrence.start));
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> CalendarEvent {
        CalendarEvent {
            id: 42,
            title: "Review".into(),
            date: "2026-11-23".into(),
            time: "09:00".into(),
            duration_minutes: 60,
            reminder_minutes: Some(15),
            repeat: crate::CalendarRepeat::Daily,
            ..CalendarEvent::default()
        }
    }
    fn now() -> i64 {
        crate::days_from_date(2026, 11, 23) * 86_400 + 8 * 3_600 + 45 * 60
    }

    #[test]
    fn recurring_lead_time_is_local_deduplicated_and_exclusion_aware() {
        let mut clock = ReminderSchedule::default();
        let mut event = event();
        assert_eq!(clock.poll(&[event.clone()], now() - 3_600, 3_600).len(), 1);
        assert!(
            clock
                .poll(&[event.clone()], now() - 3_600 + 20, 3_600)
                .is_empty()
        );
        event.excluded_dates.push("2026-11-24".into());
        assert!(clock.poll(&[event.clone()], now() + 86_400, 0).is_empty());
        assert_eq!(clock.poll(&[event], now() + 2 * 86_400, 0).len(), 1);
    }
    #[test]
    fn all_day_week_ahead_and_snooze_cancellation_use_current_event() {
        let mut clock = ReminderSchedule::default();
        let mut event = event();
        event.all_day = true;
        event.reminder_minutes = Some(10_080);
        let now = crate::days_from_date(2026, 11, 16) * 86_400;
        let due = clock.poll(&[event.clone()], now, 0);
        assert_eq!(due.len(), 1);
        clock.snooze(due[0].clone(), now, 5);
        assert!(clock.poll(&[event.clone()], now + 299, 0).is_empty());
        assert_eq!(clock.poll(&[event.clone()], now + 300, 0).len(), 1);
        clock.snooze(due[0].clone(), now + 300, 5);
        event.reminder_minutes = None;
        assert!(clock.poll(&[event], now + 600, 0).is_empty());
    }
    #[test]
    fn snoozed_reminders_refresh_edited_appointment_metadata() {
        let mut clock = ReminderSchedule::default();
        let mut event = event();
        let due = clock.poll(&[event.clone()], now(), 0);
        clock.snooze(due[0].clone(), now(), 5);
        event.title = "Updated review".into();
        event.location = "New room".into();
        event.duration_minutes = 90;
        let due = clock.poll(&[event], now() + 300, 0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].title, "Updated review");
        assert_eq!(due[0].location, "New room");
        assert_eq!(due[0].occurrence.end - due[0].occurrence.start, 90 * 60);
    }
    #[test]
    fn invalidation_finds_due_now_edits_and_preserves_fired_and_snoozed_occurrences() {
        let mut clock = ReminderSchedule::default();
        let mut events = vec![event()];
        let first = clock.poll(&events, now(), 0);
        clock.snooze(first[0].clone(), now() + 10, 5);
        events.push(CalendarEvent { id: 43, ..event() });
        assert!(clock.poll(&events, now() + 15, 0).is_empty());
        clock.invalidate();
        let due = clock.poll(&events, now() + 20, 0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].occurrence.event_id, 43);
        clock.invalidate();
        assert!(clock.poll(&events, now() + 30, 0).is_empty());
        let due = clock.poll(&events, now() + 310, 0);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].occurrence.event_id, 42);
    }
    #[test]
    fn reminder_queries_stay_inside_supported_calendar_dates() {
        let mut clock = ReminderSchedule::default();
        let events = [event()];
        for invalid in [
            i64::MIN,
            i64::MAX,
            crate::days_from_date(10000, 1, 1) * 86_400 + 8 * 3_600 + 45 * 60,
        ] {
            assert!(clock.poll(&events, invalid, 0).is_empty());
        }
        assert_eq!(clock.poll(&events, now(), 0).len(), 1);
        let last_day = crate::days_from_date(9999, 12, 31);
        let mut all_day = event();
        all_day.all_day = true;
        all_day.reminder_minutes = Some(1_440);
        assert!(clock.poll(&[all_day], last_day * 86_400, 0).is_empty());
    }
}
