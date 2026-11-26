use super::AppRegistry;
use crate::Capability;
use phone_domain::{days_from_date, validate_app_path, AppId};
use phone_state::PhoneViewModel;
use std::collections::BTreeSet;

pub const MAX_PROVIDER_REMINDERS: usize = 512;
pub const MAX_APP_REMINDERS: usize = 4096;

/// One minute-aligned local-wall-clock reminder. The host's current UTC offset
/// is applied when polling, matching Calendar's existing wall-clock model.
/// `key` identifies an app-owned item; `revision` changes when its content or
/// eligibility changes. Delivery deduplication uses owner/key/scheduled time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppReminder {
    pub key: String,
    pub revision: u64,
    pub scheduled_local: i64,
    pub title: String,
    pub detail: String,
    pub path: String,
}

impl AppReminder {
    fn validate(&self) -> Result<(), String> {
        if self.key.is_empty() || self.key.len() > 128 || self.key.chars().any(char::is_control) {
            return Err("A reminder needs a stable key of 1–128 bytes.".into());
        }
        if self.revision == 0
            || self.title.trim().is_empty()
            || self.title.len() > 1024
            || self.title.chars().any(char::is_control)
            || self.detail.len() > 4096
            || self.detail.contains('\0')
        {
            return Err("The reminder revision, title or detail is invalid.".into());
        }
        if self.scheduled_local.rem_euclid(60) != 0
            || !(days_from_date(1, 1, 1)..=days_from_date(9999, 12, 31))
                .contains(&self.scheduled_local.div_euclid(86_400))
        {
            return Err("Reminder time must be a calendar minute between years 1 and 9999.".into());
        }
        validate_app_path(&self.path)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedAppReminder {
    pub app_id: &'static str,
    pub app_title: &'static str,
    pub builtin: Option<AppId>,
    pub document_epoch: u64,
    pub reminder: AppReminder,
}

impl ScopedAppReminder {
    pub fn identity(&self) -> (&'static str, &str) {
        (self.app_id, &self.reminder.key)
    }
}

/// Invalid providers are isolated: their schedules are excluded as a unit,
/// without hiding valid schedules from other apps or rewriting stored data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppReminderSnapshot {
    pub document_epoch: u64,
    pub document_revision: u64,
    pub reminders: Vec<ScopedAppReminder>,
    pub errors: Vec<(&'static str, String)>,
}

impl AppRegistry {
    pub fn reminder_snapshot(&self, vm: &PhoneViewModel) -> AppReminderSnapshot {
        let mut snapshot = AppReminderSnapshot {
            document_epoch: vm.document_epoch(),
            document_revision: vm.revision(),
            reminders: Vec::new(),
            errors: Vec::new(),
        };
        // Registry keys, followed by each provider's stable keys, determine the
        // queue order regardless of registration or persisted collection order.
        for (id, index) in &self.ids {
            let entry = &self.entries[*index];
            let Some(provider) = entry.reminders else {
                continue;
            };
            let manifest = entry.manifest;
            if !manifest.capabilities.contains(&Capability::Notifications)
                && !manifest.capabilities.contains(&Capability::All)
            {
                continue;
            }
            let available = manifest.builtin.map_or_else(
                || vm.extension_enabled(id),
                |app| vm.data().installed.contains(&app),
            );
            if !available {
                continue;
            }
            let result = provider(vm).and_then(|mut reminders| {
                if reminders.len() > MAX_PROVIDER_REMINDERS
                    || snapshot.reminders.len() + reminders.len() > MAX_APP_REMINDERS
                {
                    return Err(
                        "The foreground reminder schedule exceeds its bounded capacity.".into(),
                    );
                }
                let mut keys = BTreeSet::new();
                for reminder in &reminders {
                    reminder.validate()?;
                    if !keys.insert(&reminder.key) {
                        return Err("A provider returned duplicate reminder keys.".into());
                    }
                    // Built-ins must admit the path through their typed parser.
                    // Extensions retain the same bounded path contract as AppContext.
                    entry.launch_payload(&reminder.path)?;
                }
                reminders.sort_by(|left, right| left.key.cmp(&right.key));
                Ok(reminders)
            });
            if vm.document_epoch() != snapshot.document_epoch
                || vm.revision() != snapshot.document_revision
            {
                snapshot.reminders.clear();
                snapshot.errors.push((
                    manifest.id,
                    "The phone document changed while collecting reminders.".into(),
                ));
                return snapshot;
            }
            match result {
                Ok(reminders) => snapshot
                    .reminders
                    .extend(reminders.into_iter().map(|reminder| ScopedAppReminder {
                        app_id: manifest.id,
                        app_title: manifest.title,
                        builtin: manifest.builtin,
                        document_epoch: snapshot.document_epoch,
                        reminder,
                    })),
                Err(error) => snapshot
                    .errors
                    .push((manifest.id, error.chars().take(1024).collect())),
            }
        }
        snapshot
    }
}

#[cfg(test)]
mod tests;
