//! Backward-compatible system preferences and bounded imported settings.
use super::Settings;
use crate::AppId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationPreference {
    pub action_center: bool,
    pub banners: bool,
    pub sound: bool,
    pub vibrate: bool,
}

impl Default for NotificationPreference {
    fn default() -> Self {
        Self {
            action_center: true,
            banners: true,
            sound: true,
            vibrate: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateOrder {
    MonthDayYear,
    #[default]
    DayMonthYear,
    YearMonthDay,
}

impl DateOrder {
    pub fn format(self, year: i32, month: u32, day: u32) -> String {
        match self {
            Self::MonthDayYear => format!("{month:02}/{day:02}/{year:04}"),
            Self::DayMonthYear => format!("{day:02}/{month:02}/{year:04}"),
            Self::YearMonthDay => format!("{year:04}-{month:02}-{day:02}"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MonthDayYear => "month / day / year",
            Self::DayMonthYear => "day / month / year",
            Self::YearMonthDay => "year - month - day",
        }
    }
}

impl Settings {
    /// Normalize only the additional preferences. PhoneData owns legacy validation.
    pub fn normalize(&mut self) {
        self.ringer_volume = self.ringer_volume.min(10);
        self.lock_timeout = match self.lock_timeout {
            0 | 30 | 60 | 180 | 300 | 600 | 900 => self.lock_timeout,
            _ => 60,
        };
        self.phone_name = self.phone_name.trim().chars().take(80).collect();
        if self.phone_name.is_empty() {
            self.phone_name = "Lumia".into();
        }
        if self.lock_wallpaper.len() > 4096 {
            self.lock_wallpaper.clear();
        }
        if !lock_status_apps()
            .iter()
            .any(|app| app.label() == self.lock_detail_app)
            && !self.lock_detail_app.is_empty()
        {
            self.lock_detail_app = "Calendar".into();
        }
        let mut selected = BTreeSet::new();
        for app in &mut self.lock_quick_apps {
            if (!app.is_empty() && !lock_status_apps().iter().any(|value| value.label() == app))
                || (!app.is_empty() && !selected.insert(app.clone()))
            {
                app.clear();
            }
        }
        self.app_notifications
            .retain(|key, _| AppId::ALL.iter().any(|app| app.label() == key));
        if !["United Kingdom", "United States", "Poland"].contains(&self.region.as_str()) {
            self.region = "United Kingdom".into();
        }
    }

    pub fn notification_preference(&self, app: AppId) -> NotificationPreference {
        self.app_notifications
            .get(app.label())
            .copied()
            .unwrap_or_default()
    }

    pub fn notifications_allowed(&self, app: AppId) -> bool {
        self.notifications && self.notification_preference(app).action_center
    }

    pub fn media_gain(&self) -> f64 {
        if self.media_muted {
            0.0
        } else {
            f64::from(self.volume.min(30)) / 30.0
        }
    }

    pub fn motion_enabled(&self) -> bool {
        !self.reduced_motion && !self.battery_saver
    }

    pub fn live_tiles_enabled(&self) -> bool {
        self.live_tiles && self.motion_enabled()
    }

    pub fn lock_app(&self, name: &str) -> Option<AppId> {
        lock_status_apps()
            .into_iter()
            .find(|app| app.label() == name)
    }
}

pub fn lock_status_apps() -> [AppId; 6] {
    [
        AppId::Phone,
        AppId::Messaging,
        AppId::Outlook,
        AppId::Calendar,
        AppId::Alarms,
        AppId::OneNote,
    ]
}
