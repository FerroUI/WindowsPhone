//! Calendar's app-owned task library and foreground-reminder provider.
mod commands;
mod editor;
mod list;
mod model;
mod state;
mod store;
mod view;

use phone_sdk::{session::Session, AppReminder};
use phone_state::PhoneViewModel;
pub use state::State;
use std::rc::Rc;
pub use view::view;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "calendar", rename_all = "snake_case", deny_unknown_fields)]
enum Activation {
    Tasks,
    Task { id: u32 },
}

pub fn launch_path(path: &str) -> Result<serde_json::Value, String> {
    let activation = if path == "tasks" {
        Activation::Tasks
    } else {
        let id = path
            .strip_prefix("task/")
            .ok_or("Unknown Calendar launch path.")?;
        let value: u32 = id.parse().map_err(|_| "Invalid task identifier.")?;
        if value == 0 || value == u32::MAX || value.to_string() != id {
            return Err("Invalid task identifier.".into());
        }
        Activation::Task { id: value }
    };
    serde_json::to_value(activation).map_err(|error| error.to_string())
}

pub fn activate(
    vm: &Rc<PhoneViewModel>,
    ui: &Rc<Session>,
    payload: serde_json::Value,
) -> Result<(), String> {
    let state = ui.state::<State>();
    if state.editor.get_untracked().is_some() && state.dirty() {
        return Err("Save or discard the task being edited before opening another item.".into());
    }
    if payload.get("calendar").is_none() {
        let occurrence: phone_domain::CalendarOccurrence = serde_json::from_value(payload)
            .map_err(|error| format!("Invalid Calendar activation: {error}"))?;
        state.reset();
        ui.state::<crate::CalendarSession>()
            .open_occurrence(occurrence);
        return Ok(());
    }
    let activation: Activation = serde_json::from_value(payload)
        .map_err(|error| format!("Invalid Calendar task activation: {error}"))?;
    if ui
        .state::<crate::CalendarSession>()
        .editor
        .get_untracked()
        .is_some()
    {
        return Err("Finish editing the appointment before opening a task.".into());
    }
    let id = match activation {
        Activation::Tasks => None,
        Activation::Task { id } => {
            if id == 0 || store::read(vm)?.task(id).is_none() {
                return Err("This task no longer exists.".into());
            }
            Some(id)
        }
    };
    ui.state::<crate::CalendarSession>().reset_transient();
    state.close_editor();
    state.close_overlays();
    state.active.set(true);
    state.selected.set(id);
    ui.menu.set(false);
    Ok(())
}

pub fn open(ui: &Rc<Session>) {
    ui.menu.set(false);
    ui.state::<State>().active.set(true);
}

pub fn back(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    let state = ui.state::<State>();
    if !state.active.get_untracked() {
        return false;
    }
    if state.context.get_untracked().is_some() {
        state.context.set(None);
    } else if state.choice.get_untracked().is_some() {
        state.choice.set(None);
    } else if let Some(edit) = state.editor.get_untracked() {
        if state.dirty() {
            let close = ui.clone();
            let model = vm.clone();
            ui.form_with_action(
                "discard unsaved task changes?",
                vec![],
                "discard",
                Rc::new(move |_| {
                    if close.state::<State>().matches(&model, &edit) {
                        close.state::<State>().close_editor();
                    }
                }),
            );
        } else {
            state.close_editor();
        }
    } else if state.selected.get_untracked().is_some() {
        state.selected.set(None);
    } else if state.searching.get_untracked() {
        state.searching.set(false);
        state.query.set(String::new());
    } else {
        state.active.set(false);
    }
    true
}

pub fn reminders(vm: &PhoneViewModel) -> Result<Vec<AppReminder>, String> {
    let library = match vm.app_data(store::APP_ID) {
        Some(value) => {
            serde_json::from_value::<model::Library>(value).map_err(|error| error.to_string())?
        }
        None => model::Library::default(),
    };
    library.validate()?;
    library
        .tasks
        .into_iter()
        .filter(|task| !task.completed())
        .filter_map(|task| {
            task.fields.reminder.as_ref().map(|reminder| {
                Ok(AppReminder {
                    key: format!("task/{}", task.id),
                    revision: u64::from(task.revision),
                    scheduled_local: reminder.local_timestamp()?,
                    title: task.fields.title.clone(),
                    detail: task
                        .fields
                        .due
                        .as_ref()
                        .map(|date| {
                            format!(
                                "due {} · {} priority",
                                crate::long_date(date),
                                task.fields.priority.label()
                            )
                        })
                        .unwrap_or_else(|| format!("{} priority", task.fields.priority.label())),
                    path: format!("task/{}", task.id),
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
