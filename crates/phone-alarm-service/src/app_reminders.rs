//! Foreground-only schedules supplied by registered applications. Wall-clock
//! minutes match Calendar; snoozes retain a UTC deadline across offset changes.
use phone_sdk::{AppReminderSnapshot, ScopedAppReminder, MAX_APP_REMINDERS};
use std::collections::BTreeMap;

type Key = (&'static str, String);

#[derive(Default)]
pub(super) struct Schedule {
    pub tag: Option<(u64, u64)>,
    current: BTreeMap<Key, ScopedAppReminder>,
    last_minute: Option<i64>,
    fired: BTreeMap<Key, i64>,
    snoozed: BTreeMap<Key, (i64, i64)>,
}

impl Schedule {
    pub fn replace(&mut self, snapshot: AppReminderSnapshot) {
        if self
            .tag
            .is_some_and(|(epoch, _)| epoch != snapshot.document_epoch)
        {
            self.clear();
        }
        self.tag = Some((snapshot.document_epoch, snapshot.document_revision));
        self.current = snapshot
            .reminders
            .into_iter()
            .map(|item| ((item.app_id, item.reminder.key.clone()), item))
            .collect();
        self.last_minute = None;
        self.snoozed.retain(|key, (scheduled, _)| {
            self.current
                .get(key)
                .is_some_and(|item| item.reminder.scheduled_local == *scheduled)
        });
    }

    pub fn current(&self, previous: &ScopedAppReminder) -> Option<ScopedAppReminder> {
        self.current
            .get(&(previous.app_id, previous.reminder.key.clone()))
            .filter(|item| {
                item.document_epoch == previous.document_epoch
                    && item.reminder.scheduled_local == previous.reminder.scheduled_local
            })
            .cloned()
    }

    pub fn poll(&mut self, now: i64, offset: i32) -> Vec<ScopedAppReminder> {
        let local = now.saturating_add(i64::from(offset));
        let day = local.div_euclid(86_400);
        if !(phone_domain::days_from_date(1, 1, 1)..=phone_domain::days_from_date(9999, 12, 31))
            .contains(&day)
        {
            return Vec::new();
        }
        let minute = local.div_euclid(60);
        let changed = self.last_minute.replace(minute) != Some(minute);
        let ready = self
            .snoozed
            .iter()
            .filter_map(|(key, (_, due))| (now >= *due).then(|| key.clone()))
            .collect::<Vec<_>>();
        let mut due = ready
            .into_iter()
            .filter_map(|key| {
                let (scheduled, _) = self.snoozed.remove(&key)?;
                self.current
                    .get(&key)
                    .filter(|item| item.reminder.scheduled_local == scheduled)
                    .cloned()
            })
            .collect::<Vec<_>>();
        if !changed {
            return due;
        }

        // Keep high-water marks for live items, plus retired items in this
        // minute. Complete/reopen and disable/enable cannot replay that same
        // due instant. Monotonic app identities bound normal history to the
        // current schedule; a burst of deleted/new identities is capped too.
        self.fired.retain(|key, scheduled| {
            self.current.contains_key(key) || scheduled.div_euclid(60) >= minute
        });
        for (key, item) in &self.current {
            let scheduled = item.reminder.scheduled_local;
            if scheduled.div_euclid(60) != minute
                || self
                    .fired
                    .get(key)
                    .is_some_and(|previous| *previous >= scheduled)
                || self.snoozed.contains_key(key)
            {
                continue;
            }
            if self.fired.len() >= MAX_APP_REMINDERS * 2 && !self.fired.contains_key(key) {
                // Never evict a still-relevant fired key and replay it. The
                // next minute releases retired burst entries automatically.
                continue;
            }
            self.fired.insert(key.clone(), scheduled);
            due.push(item.clone());
        }
        due
    }

    pub fn snooze(&mut self, item: &ScopedAppReminder, now: i64, minutes: u8) {
        let key = (item.app_id, item.reminder.key.clone());
        if self.current(item).is_none()
            || (self.snoozed.len() >= MAX_APP_REMINDERS && !self.snoozed.contains_key(&key))
        {
            return;
        }
        self.snoozed.insert(
            key,
            (
                item.reminder.scheduled_local,
                now.saturating_add(i64::from(minutes.clamp(1, 60)) * 60),
            ),
        );
    }

    pub fn cancel(&mut self, item: &ScopedAppReminder) {
        self.snoozed
            .remove(&(item.app_id, item.reminder.key.clone()));
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests;
