//! Bounded, transient alarm scheduling shared by the browser and desktop hosts.
use crate::{Alarm, Id};
use std::collections::BTreeMap;

pub const MAX_ALARMS: usize = 256;
pub const ALARM_SNOOZE_MINUTES: [u8; 5] = [5, 10, 20, 30, 60];
pub const fn default_alarm_snooze() -> u8 {
    5
}

#[derive(Clone, Debug)]
struct Snoozed {
    alarm: Alarm,
    due: i64,
}

/// Scheduling is ephemeral: advancing time never writes the phone document.
/// A host calls `poll` while running. Missed background minutes are not replayed.
#[derive(Default, Debug)]
pub struct AlarmSchedule {
    last_minute: Option<i64>,
    fired: BTreeMap<Id, i64>,
    snoozed: BTreeMap<Id, Snoozed>,
}

impl AlarmSchedule {
    pub fn poll(&mut self, alarms: &[Alarm], now: i64, utc_offset: i32) -> Vec<Alarm> {
        let current = |id| alarms.iter().find(|alarm| alarm.id == id);
        self.fired.retain(|id, _| current(*id).is_some());
        self.snoozed.retain(|id, pending| {
            current(*id).is_some_and(|alarm| {
                (alarm.enabled || pending.alarm.weekdays.is_empty())
                    && alarm.hour == pending.alarm.hour
                    && alarm.minute == pending.alarm.minute
                    && alarm.weekdays == pending.alarm.weekdays
            })
        });
        let ready = self
            .snoozed
            .iter()
            .filter_map(|(id, pending)| (now >= pending.due).then_some(*id))
            .collect::<Vec<_>>();
        let mut due = Vec::new();
        for id in ready {
            self.snoozed.remove(&id);
            if let Some(alarm) = current(id) {
                due.push(alarm.clone());
            }
        }
        let local = now.saturating_add(i64::from(utc_offset));
        let minute = local.div_euclid(60);
        if self.last_minute.replace(minute) == Some(minute) {
            return due;
        }
        let weekday = (local.div_euclid(86_400) + 3).rem_euclid(7) as u8;
        let clock = minute.rem_euclid(1_440);
        for alarm in alarms.iter().take(MAX_ALARMS) {
            if alarm.enabled
                && i64::from(alarm.hour) * 60 + i64::from(alarm.minute) == clock
                && (alarm.weekdays.is_empty() || alarm.weekdays.contains(&weekday))
                && !due.iter().any(|ready| ready.id == alarm.id)
                && self.mark_fired(alarm.id, now, utc_offset)
            {
                due.push(alarm.clone());
            }
        }
        due
    }

    /// Recheck changed definitions this minute without replaying delivered alarms.
    pub fn invalidate(&mut self) {
        self.last_minute = None;
    }

    /// Claim a scheduled delivery, including legacy host callbacks. A false result
    /// means that this alarm has already fired in this minute or a later minute.
    pub fn mark_fired(&mut self, id: Id, now: i64, utc_offset: i32) -> bool {
        let minute = now.saturating_add(i64::from(utc_offset)).div_euclid(60);
        if self.fired.get(&id).is_some_and(|last| *last >= minute)
            || (self.fired.len() >= MAX_ALARMS && !self.fired.contains_key(&id))
        {
            return false;
        }
        self.fired.insert(id, minute);
        self.snoozed.remove(&id);
        true
    }

    pub fn snooze(&mut self, alarm: Alarm, now: i64, minutes: u8) {
        if self.snoozed.len() >= MAX_ALARMS && !self.snoozed.contains_key(&alarm.id) {
            return;
        }
        self.snoozed.insert(
            alarm.id,
            Snoozed {
                alarm,
                due: now.saturating_add(i64::from(minutes.clamp(1, 60)) * 60),
            },
        );
    }

    pub fn cancel(&mut self, id: Id) {
        self.snoozed.remove(&id);
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn alarm() -> Alarm {
        Alarm {
            id: 7,
            hour: 7,
            enabled: true,
            ..Alarm::default()
        }
    }
    fn monday() -> i64 {
        crate::days_from_date(2026, 11, 23) * 86_400 + 7 * 3_600
    }

    #[test]
    fn current_minute_fires_once_and_clock_reversal_does_not_repeat() {
        let mut clock = AlarmSchedule::default();
        let data = [alarm()];
        let now = monday();
        assert_eq!(clock.poll(&data, now, 0).len(), 1);
        assert!(clock.poll(&data, now + 30, 0).is_empty());
        assert!(clock.poll(&data, now - 60, 0).is_empty());
        assert!(clock.poll(&data, now, 0).is_empty());
        assert_eq!(clock.poll(&data, now + 86_400, 0).len(), 1);
    }
    #[test]
    fn local_weekdays_and_offset_are_used_without_replaying_missed_minutes() {
        let mut clock = AlarmSchedule::default();
        let mut item = alarm();
        item.weekdays = vec![0];
        assert_eq!(
            clock.poll(&[item.clone()], monday() - 3_600, 3_600).len(),
            1
        );
        assert!(clock.poll(&[item.clone()], monday() + 86_400, 0).is_empty());
        let mut fresh = AlarmSchedule::default();
        assert!(fresh.poll(&[item], monday() + 61, 0).is_empty());
    }
    #[test]
    fn once_snooze_survives_consumption_but_not_deletion_or_time_edit() {
        let mut clock = AlarmSchedule::default();
        let mut item = alarm();
        let now = monday();
        clock.poll(&[item.clone()], now, 0);
        clock.snooze(item.clone(), now + 10, 5);
        item.enabled = false;
        assert!(clock.poll(&[item.clone()], now + 309, 0).is_empty());
        assert_eq!(clock.poll(&[item.clone()], now + 310, 0).len(), 1);
        clock.snooze(item.clone(), now + 310, 2);
        item.hour = 8;
        assert!(clock.poll(&[item.clone()], now + 430, 0).is_empty());
        clock.snooze(item, now + 500, 2);
        assert!(clock.poll(&[], now + 620, 0).is_empty());
    }
    #[test]
    fn repeating_alarm_disable_cancels_pending_snooze() {
        let mut clock = AlarmSchedule::default();
        let mut item = alarm();
        item.weekdays = vec![0];
        clock.snooze(item.clone(), monday(), 5);
        item.enabled = false;
        assert!(clock.poll(&[item], monday() + 300, 0).is_empty());
    }
    #[test]
    fn invalidation_finds_new_due_alarms_without_replaying_or_losing_snoozes() {
        let mut clock = AlarmSchedule::default();
        let first = alarm();
        let now = monday();
        let mut alarms = vec![first.clone()];
        assert_eq!(clock.poll(&alarms, now, 0).len(), 1);
        clock.snooze(first, now + 10, 5);
        alarms.push(Alarm { id: 8, ..alarm() });
        assert!(clock.poll(&alarms, now + 15, 0).is_empty());
        clock.invalidate();
        let due = clock.poll(&alarms, now + 20, 0);
        assert_eq!(due.iter().map(|item| item.id).collect::<Vec<_>>(), vec![8]);
        clock.invalidate();
        assert!(clock.poll(&alarms, now + 30, 0).is_empty());
        let snoozed = clock.poll(&alarms, now + 310, 0);
        assert_eq!(
            snoozed.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![7]
        );
    }
    #[test]
    fn legacy_delivery_claims_share_poll_deduplication_without_hiding_other_alarms() {
        let mut clock = AlarmSchedule::default();
        let now = monday() - 3_600;
        let alarms = [alarm(), Alarm { id: 8, ..alarm() }];
        assert!(clock.mark_fired(7, now, 3_600));
        assert!(!clock.mark_fired(7, now + 30, 3_600));
        assert!(!clock.mark_fired(7, now - 60, 3_600));
        let due = clock.poll(&alarms, now, 3_600);
        assert_eq!(due.iter().map(|item| item.id).collect::<Vec<_>>(), vec![8]);
        assert!(!clock.mark_fired(8, now, 3_600));
        assert_eq!(clock.poll(&alarms, now + 86_400, 3_600).len(), 2);
    }
    #[test]
    fn old_alarm_documents_default_to_five_minute_snooze() {
        let item: Alarm = serde_json::from_str(
            r#"{"id":7,"label":"Morning","hour":7,"minute":0,"enabled":true,"weekdays":[]}"#,
        )
        .unwrap();
        assert_eq!(item.snooze_minutes, 5);
        assert_eq!(Alarm::default().snooze_minutes, 5);
    }
}
