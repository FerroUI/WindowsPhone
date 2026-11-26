use super::model::{Fields, Filter, Priority, Reminder, Sort, Task};
use crate::session::{date_value, stored_date};
use ferroui_base::{animation::SignedDuration, utilities::DateTimeOffset};
use ferroui_view::prelude::*;
use phone_domain::{calendar_date, calendar_minute};
use phone_state::PhoneViewModel;
use std::cell::Cell;

#[derive(Clone)]
pub struct Edit {
    pub original: Option<Task>,
    pub epoch: u64,
    pub generation: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Filter,
    Sort,
    Priority,
}

pub struct State {
    pub active: Signal<bool>,
    pub selected: Signal<Option<u32>>,
    pub editor: Signal<Option<Edit>>,
    pub choice: Signal<Option<Choice>>,
    pub context: Signal<Option<(u64, Task)>>,
    pub filter: Signal<Filter>,
    pub sort: Signal<Sort>,
    pub query: Signal<String>,
    pub searching: Signal<bool>,
    pub subject: Signal<String>,
    pub notes: Signal<String>,
    pub due_enabled: Signal<bool>,
    pub due: Signal<Option<DateTimeOffset>>,
    pub priority: Signal<Priority>,
    pub reminder_enabled: Signal<bool>,
    pub reminder_date: Signal<Option<DateTimeOffset>>,
    pub reminder_time: Signal<Option<SignedDuration>>,
    pub error: Signal<String>,
    generation: Cell<u64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: signal(false),
            selected: signal(None),
            editor: signal(None),
            choice: signal(None),
            context: signal(None),
            filter: signal(Filter::Active),
            sort: signal(Sort::DueDate),
            query: signal(String::new()),
            searching: signal(false),
            subject: signal(String::new()),
            notes: signal(String::new()),
            due_enabled: signal(false),
            due: signal(None),
            priority: signal(Priority::Normal),
            reminder_enabled: signal(false),
            reminder_date: signal(None),
            reminder_time: signal(None),
            error: signal(String::new()),
            generation: Cell::new(0),
        }
    }
}

impl State {
    pub fn begin(&self, vm: &PhoneViewModel, original: Option<Task>) {
        let fields = original
            .as_ref()
            .map(|task| task.fields.clone())
            .unwrap_or_default();
        let today = calendar_date(vm.local_now().div_euclid(86_400));
        self.generation.set(self.generation.get().wrapping_add(1));
        self.subject.set(fields.title);
        self.notes.set(fields.notes);
        self.due_enabled.set(fields.due.is_some());
        self.due
            .set(date_value(fields.due.as_deref().unwrap_or(&today)));
        self.priority.set(fields.priority);
        self.reminder_enabled.set(fields.reminder.is_some());
        let next_minute_date = calendar_date(vm.local_now().saturating_add(60).div_euclid(86_400));
        self.reminder_date.set(date_value(
            fields
                .reminder
                .as_ref()
                .map(|item| item.date.as_str())
                .unwrap_or(&next_minute_date),
        ));
        let minute = fields
            .reminder
            .as_ref()
            .and_then(|item| calendar_minute(&item.time))
            .unwrap_or(((vm.local_now().rem_euclid(86_400) / 60 + 1) % 1440) as u32);
        self.reminder_time
            .set(Some(SignedDuration::from_minutes(f64::from(minute))));
        self.error.set(String::new());
        self.close_overlays();
        self.active.set(true);
        self.editor.set(Some(Edit {
            original,
            epoch: vm.document_epoch(),
            generation: self.generation.get(),
        }));
    }

    pub fn fields(&self) -> Result<Fields, String> {
        let due = if self.due_enabled.get_untracked() {
            Some(stored_date(
                self.due.get_untracked().ok_or("Choose a due date.")?,
            ))
        } else {
            None
        };
        let reminder = if self.reminder_enabled.get_untracked() {
            let date = stored_date(
                self.reminder_date
                    .get_untracked()
                    .ok_or("Choose a reminder date.")?,
            );
            let time = self
                .reminder_time
                .get_untracked()
                .ok_or("Choose a reminder time.")?;
            let minute = time.total_minutes();
            if !minute.is_finite() || !(0.0..1440.0).contains(&minute) || minute.fract() != 0.0 {
                return Err("Choose a valid reminder time.".into());
            }
            let minute = minute as u32;
            Some(Reminder {
                date,
                time: format!("{:02}:{:02}", minute / 60, minute % 60),
            })
        } else {
            None
        };
        Ok(Fields {
            title: self.subject.get_untracked().trim().into(),
            notes: self.notes.get_untracked(),
            due,
            priority: self.priority.get_untracked(),
            reminder,
        })
    }

    pub fn dirty(&self) -> bool {
        let Some(edit) = self.editor.get_untracked() else {
            return false;
        };
        let original = edit
            .original
            .as_ref()
            .map(|task| task.fields.clone())
            .unwrap_or_default();
        self.fields().map_or(true, |fields| fields != original)
    }

    pub fn matches(&self, vm: &PhoneViewModel, edit: &Edit) -> bool {
        vm.document_epoch() == edit.epoch
            && self.editor.get_untracked().is_some_and(|current| {
                current.epoch == edit.epoch && current.generation == edit.generation
            })
    }

    pub fn close_editor(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.editor.set(None);
        self.choice.set(None);
        self.error.set(String::new());
    }

    pub fn close_overlays(&self) {
        self.context.set(None);
        self.choice.set(None);
    }

    pub fn reset(&self) {
        self.close_editor();
        self.close_overlays();
        self.active.set(false);
        self.selected.set(None);
        self.filter.set(Filter::Active);
        self.sort.set(Sort::DueDate);
        self.query.set(String::new());
        self.searching.set(false);
        self.subject.set(String::new());
        self.notes.set(String::new());
        self.due.set(None);
        self.reminder_date.set(None);
        self.reminder_time.set(None);
    }
}
