//! Bounded local Calendar tasks. Dates are calendar dates, not UTC deadlines.
use phone_domain::{calendar_date, calendar_day, calendar_minute, days_from_date};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BTreeSet};

pub const MAX_TASKS: usize = 512;
pub const MAX_NOTES_BYTES: usize = 16 * 1024;
const MAX_SAFE_TIMESTAMP: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl Priority {
    pub const ALL: [Self; 3] = [Self::Low, Self::Normal, Self::High];

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    pub title: String,
    pub notes: String,
    pub due: Option<String>,
    pub priority: Priority,
    pub reminder: Option<Reminder>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reminder {
    pub date: String,
    pub time: String,
}

impl Reminder {
    pub fn local_timestamp(&self) -> Result<i64, String> {
        let day = calendar_day(&self.date).ok_or("Choose a valid reminder date.")?;
        let minute = calendar_minute(&self.time).ok_or("Choose a valid reminder time.")?;
        if calendar_date(day) != self.date
            || format!("{:02}:{:02}", minute / 60, minute % 60) != self.time
        {
            return Err("Choose a valid reminder date and time.".into());
        }
        Ok(day * 86_400 + i64::from(minute) * 60)
    }
}

impl Fields {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty()
            || self.title.trim() != self.title
            || self.title.chars().count() > 160
            || self.title.chars().any(char::is_control)
        {
            return Err("Enter a task subject of 1–160 characters on one line.".into());
        }
        if self.notes.len() > MAX_NOTES_BYTES || self.notes.contains('\0') {
            return Err(
                "Task notes must fit within 16 KiB and cannot contain null characters.".into(),
            );
        }
        if let Some(due) = &self.due {
            let day = calendar_day(due).ok_or("Choose a valid due date.")?;
            if calendar_date(day) != *due {
                return Err("Choose a valid due date.".into());
            }
        }
        if let Some(reminder) = &self.reminder {
            reminder.local_timestamp()?;
        }
        Ok(())
    }

    pub fn due_today(&self, today: i64) -> Result<Self, String> {
        self.with_due_day(today, today)
    }

    pub fn postponed(&self, today: i64) -> Result<Self, String> {
        let previous = self
            .due
            .as_deref()
            .and_then(calendar_day)
            .or_else(|| {
                self.reminder
                    .as_ref()
                    .and_then(|item| calendar_day(&item.date))
            })
            .unwrap_or(today);
        self.with_due_day(
            previous
                .checked_add(1)
                .ok_or("The due date is out of range.")?,
            today,
        )
    }

    fn with_due_day(&self, day: i64, today: i64) -> Result<Self, String> {
        if !(days_from_date(1, 1, 1)..=days_from_date(9999, 12, 31)).contains(&day) {
            return Err("The due date must be between years 1 and 9999.".into());
        }
        let mut fields = self.clone();
        let previous = self
            .due
            .as_deref()
            .and_then(calendar_day)
            .or_else(|| {
                self.reminder
                    .as_ref()
                    .and_then(|item| calendar_day(&item.date))
            })
            .unwrap_or(today);
        // Rescheduling carries an optional reminder by the same number of
        // calendar days, retaining its wall-clock time and lead-day interval.
        if let Some(reminder) = &mut fields.reminder {
            let reminder_day = calendar_day(&reminder.date)
                .ok_or("Choose a valid reminder date.")?
                .checked_add(day - previous)
                .ok_or("The reminder date is out of range.")?;
            if !(days_from_date(1, 1, 1)..=days_from_date(9999, 12, 31)).contains(&reminder_day) {
                return Err("The reminder date must be between years 1 and 9999.".into());
            }
            reminder.date = calendar_date(reminder_day);
        }
        fields.due = Some(calendar_date(day));
        fields.validate()?;
        Ok(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: u32,
    pub revision: u32,
    pub fields: Fields,
    pub created: i64,
    pub updated: i64,
    pub completed_at: Option<i64>,
}

impl Task {
    pub fn completed(&self) -> bool {
        self.completed_at.is_some()
    }

    pub fn overdue(&self, today: i64) -> bool {
        !self.completed()
            && self
                .fields
                .due
                .as_deref()
                .and_then(calendar_day)
                .is_some_and(|day| day < today)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub version: u32,
    pub next_id: u32,
    pub tasks: Vec<Task>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            tasks: Vec::new(),
        }
    }
}

impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This task data version is not supported.".into());
        }
        if self.next_id == 0 || self.tasks.len() > MAX_TASKS {
            return Err("The task library is invalid or exceeds 512 tasks.".into());
        }
        let mut ids = BTreeSet::new();
        for task in &self.tasks {
            if task.id == 0
                || task.id >= self.next_id
                || task.revision == 0
                || !ids.insert(task.id)
                || [Some(task.created), Some(task.updated), task.completed_at]
                    .into_iter()
                    .flatten()
                    .any(|time| time.unsigned_abs() > MAX_SAFE_TIMESTAMP)
            {
                return Err(
                    "The task library contains invalid identifiers, revisions or timestamps."
                        .into(),
                );
            }
            task.fields.validate()?;
        }
        Ok(())
    }

    pub fn task(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    pub fn rows(&self, filter: Filter, sort: Sort, query: &str, today: i64) -> Vec<Task> {
        let query = query.trim().to_lowercase();
        let mut rows = self
            .tasks
            .iter()
            .filter(|task| {
                filter.includes(task, today)
                    && (query.is_empty()
                        || task.fields.title.to_lowercase().contains(&query)
                        || task.fields.notes.to_lowercase().contains(&query))
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            // Completed tasks use completion order, including when the user
            // arrives from the active list's due-date or priority sorting.
            let status = if filter == Filter::Completed {
                Reverse(left.completed_at).cmp(&Reverse(right.completed_at))
            } else {
                left.completed().cmp(&right.completed())
            };
            let due = || {
                (left.fields.due.is_none(), &left.fields.due)
                    .cmp(&(right.fields.due.is_none(), &right.fields.due))
            };
            let priority =
                || Reverse(left.fields.priority.rank()).cmp(&Reverse(right.fields.priority.rank()));
            let title = || {
                left.fields
                    .title
                    .to_lowercase()
                    .cmp(&right.fields.title.to_lowercase())
            };
            status
                .then_with(|| match sort {
                    Sort::DueDate => due().then_with(priority).then_with(title),
                    Sort::Priority => priority().then_with(due).then_with(title),
                    Sort::Subject => title().then_with(due).then_with(priority),
                })
                .then_with(|| left.id.cmp(&right.id))
        });
        rows
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    Active,
    Due,
    Completed,
    All,
}

impl Filter {
    pub const ALL: [Self; 4] = [Self::Active, Self::Due, Self::Completed, Self::All];

    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Due => "due today",
            Self::Completed => "completed",
            Self::All => "all tasks",
        }
    }

    fn includes(self, task: &Task, today: i64) -> bool {
        match self {
            Self::Active => !task.completed(),
            Self::Due => {
                !task.completed()
                    && task
                        .fields
                        .due
                        .as_deref()
                        .and_then(calendar_day)
                        .is_some_and(|day| day <= today)
            }
            Self::Completed => task.completed(),
            Self::All => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    DueDate,
    Priority,
    Subject,
}

impl Sort {
    pub const ALL: [Self; 3] = [Self::DueDate, Self::Priority, Self::Subject];

    pub fn label(self) -> &'static str {
        match self {
            Self::DueDate => "due date",
            Self::Priority => "priority",
            Self::Subject => "subject",
        }
    }
}

#[cfg(test)]
mod tests;
