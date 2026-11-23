use ferroui_base::animation::SignedDuration;
use ferroui_view::prelude::*;
use phone_domain::Alarm;
use phone_state::PhoneViewModel;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    Main,
    Repeat,
    Snooze,
}

pub struct AlarmEditor {
    pub draft: Signal<Option<Alarm>>,
    pub time: Signal<Option<SignedDuration>>,
    pub label: Signal<String>,
    pub weekdays: Signal<Vec<u8>>,
    pub snooze: Signal<u8>,
    pub page: Signal<Picker>,
    pub error: Signal<String>,
}

impl AlarmEditor {
    pub fn new() -> Self {
        Self {
            draft: signal(None),
            time: signal(None),
            label: signal(String::new()),
            weekdays: signal(Vec::new()),
            snooze: signal(5),
            page: signal(Picker::Main),
            error: signal(String::new()),
        }
    }
    pub fn open(&self, alarm: Alarm) {
        self.time.set(Some(SignedDuration::from_minutes(
            f64::from(alarm.hour) * 60.0 + f64::from(alarm.minute),
        )));
        self.label.set(alarm.label.clone());
        self.weekdays.set(alarm.weekdays.clone());
        self.snooze.set(alarm.snooze_minutes);
        self.page.set(Picker::Main);
        self.error.set(String::new());
        self.draft.set(Some(alarm));
    }
    pub fn close(&self) {
        self.draft.set(None);
        self.page.set(Picker::Main);
        self.error.set(String::new());
    }
    pub fn dismiss(&self) -> bool {
        if self.draft.get_untracked().is_none() {
            return false;
        }
        if self.page.get_untracked() != Picker::Main {
            self.page.set(Picker::Main);
        } else {
            self.close();
        }
        true
    }
    pub fn save(&self, vm: &PhoneViewModel) -> Option<phone_domain::Id> {
        let Some(mut alarm) = self.draft.get_untracked() else {
            return None;
        };
        let Some(minutes) = self
            .time
            .get_untracked()
            .map(|time| time.total_minutes().round())
            .filter(|minutes| minutes.is_finite() && (0.0..1440.0).contains(minutes))
        else {
            self.error.set("Choose a valid time.".into());
            return None;
        };
        alarm.hour = (minutes as u16 / 60) as u8;
        alarm.minute = (minutes as u16 % 60) as u8;
        alarm.label = self.label.get_untracked().trim().to_owned();
        if alarm.label.is_empty() {
            alarm.label = "Alarm".into();
        }
        alarm.weekdays = self.weekdays.get_untracked();
        alarm.snooze_minutes = self.snooze.get_untracked();
        vm.clear_status();
        let id = alarm.id;
        vm.save_alarm(alarm);
        if vm.status().is_empty() {
            self.close();
            Some(id)
        } else {
            self.error.set(vm.status());
            None
        }
    }
}
