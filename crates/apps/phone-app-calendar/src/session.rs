use ferroui_base::{animation::SignedDuration, utilities::DateTimeOffset};
use ferroui_view::prelude::*;
use phone_domain::{
    calendar_day, calendar_minute, date_parts, CalendarEvent, CalendarOccurrence, CalendarRepeat,
    CalendarStatus,
};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarView {
    Day,
    Week,
    #[default]
    Month,
    Year,
}
impl CalendarView {
    pub const ALL: [Self; 4] = [Self::Day, Self::Week, Self::Month, Self::Year];
    pub fn label(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarPicker {
    View,
    JumpDate,
    Duration,
    Repeat,
    Reminder,
    Status,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeAction {
    Edit,
    Delete,
}

#[derive(Clone)]
pub struct CalendarEdit {
    pub original: CalendarEvent,
    pub occurrence_date: Option<String>,
}

pub struct CalendarSession {
    pub view: Signal<CalendarView>,
    pub date: Signal<String>,
    pub selected: Signal<Option<CalendarOccurrence>>,
    pub expanded_day: Signal<Option<String>>,
    pub editor: Signal<Option<CalendarEdit>>,
    pub picker: Signal<Option<CalendarPicker>>,
    pub scope: Signal<Option<ScopeAction>>,
    pub subject: Signal<String>,
    pub location: Signal<String>,
    pub notes: Signal<String>,
    pub date_value: Signal<Option<DateTimeOffset>>,
    pub time_value: Signal<Option<SignedDuration>>,
    pub duration: Signal<u32>,
    pub all_day: Signal<bool>,
    pub repeat: Signal<CalendarRepeat>,
    pub repeat_ends: Signal<bool>,
    pub until: Signal<Option<DateTimeOffset>>,
    pub reminder: Signal<Option<u32>>,
    pub status: Signal<CalendarStatus>,
    pub more: Signal<bool>,
    pub error: Signal<String>,
    pub(crate) day_scroll: RefCell<Option<(String, f64)>>,
}
impl Default for CalendarSession {
    fn default() -> Self {
        Self {
            view: signal(CalendarView::Month),
            date: signal(String::new()),
            selected: signal(None),
            expanded_day: signal(None),
            editor: signal(None),
            picker: signal(None),
            scope: signal(None),
            subject: signal(String::new()),
            location: signal(String::new()),
            notes: signal(String::new()),
            date_value: signal(None),
            time_value: signal(None),
            duration: signal(60),
            all_day: signal(false),
            repeat: signal(CalendarRepeat::Never),
            repeat_ends: signal(false),
            until: signal(None),
            reminder: signal(Some(15)),
            status: signal(CalendarStatus::Free),
            more: signal(false),
            error: signal(String::new()),
            day_scroll: RefCell::new(None),
        }
    }
}
impl CalendarSession {
    pub fn open_occurrence(&self, occurrence: CalendarOccurrence) {
        self.editor.set(None);
        self.picker.set(None);
        self.scope.set(None);
        self.date.set(occurrence.date.clone());
        self.selected.set(Some(occurrence));
    }

    pub fn begin(&self, event: CalendarEvent, occurrence_date: Option<String>) {
        self.subject.set(event.title.clone());
        self.location.set(event.location.clone());
        self.notes.set(event.details.clone());
        self.date_value.set(date_value(
            occurrence_date.as_deref().unwrap_or(&event.date),
        ));
        self.time_value.set(
            calendar_minute(&event.time).map(|minute| SignedDuration::from_minutes(minute as f64)),
        );
        self.duration.set(event.duration_minutes);
        self.all_day.set(event.all_day);
        self.repeat.set(if occurrence_date.is_some() {
            CalendarRepeat::Never
        } else {
            event.repeat
        });
        self.repeat_ends
            .set(occurrence_date.is_none() && event.repeat_until.is_some());
        self.until.set(
            event
                .repeat_until
                .as_deref()
                .and_then(date_value)
                .or_else(|| date_value(&event.date)),
        );
        self.reminder.set(event.reminder_minutes);
        self.status.set(event.status);
        self.more.set(event.id != 0);
        self.error.set(String::new());
        self.scope.set(None);
        self.picker.set(None);
        self.editor.set(Some(CalendarEdit {
            original: event,
            occurrence_date,
        }));
    }

    pub fn dismiss(&self) -> bool {
        if self.picker.get_untracked().is_some() {
            self.picker.set(None);
        } else if self.scope.get_untracked().is_some() {
            self.scope.set(None);
        } else if self.editor.get_untracked().is_some() {
            self.editor.set(None);
            self.error.set(String::new());
        } else if self.selected.get_untracked().is_some() {
            self.selected.set(None);
        } else if self.expanded_day.get_untracked().is_some() {
            self.expanded_day.set(None);
        } else {
            return false;
        }
        true
    }

    pub fn reset_transient(&self) {
        self.selected.set(None);
        self.editor.set(None);
        self.picker.set(None);
        self.scope.set(None);
        self.error.set(String::new());
    }

    pub fn reset(&self) {
        self.day_scroll.borrow_mut().take();
        self.reset_transient();
        self.view.set(CalendarView::Month);
        self.date.set(String::new());
        self.expanded_day.set(None);
        self.subject.set(String::new());
        self.location.set(String::new());
        self.notes.set(String::new());
        self.date_value.set(None);
        self.time_value.set(None);
        self.until.set(None);
        self.repeat.set(CalendarRepeat::Never);
        self.repeat_ends.set(false);
        self.all_day.set(false);
        self.duration.set(60);
        self.reminder.set(Some(15));
        self.status.set(CalendarStatus::Free);
        self.more.set(false);
    }
}

pub fn date_value(date: &str) -> Option<DateTimeOffset> {
    let (year, month, day) = date_parts(calendar_day(date)? * 86_400);
    DateTimeOffset::try_new(
        year,
        month as i32,
        day as i32,
        0,
        0,
        0,
        SignedDuration::ZERO,
    )
}

pub fn stored_date(value: DateTimeOffset) -> String {
    phone_domain::date_string(value.year(), value.month() as u32, value.day() as u32)
}
