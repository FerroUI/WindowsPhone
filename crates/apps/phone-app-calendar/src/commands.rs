use super::{
    session::{stored_date, ScopeAction},
    CalendarView,
};
use phone_domain::*;
use phone_sdk::session::Session;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn new_event(ui: &Session, date: String, minute: u32) {
    ui.state::<crate::CalendarSession>().begin(
        CalendarEvent {
            date,
            time: format!("{:02}:{:02}", minute / 60, minute % 60),
            duration_minutes: 60,
            reminder_minutes: Some(15),
            ..Default::default()
        },
        None,
    );
}

pub fn show_day(ui: &Session, date: String) {
    ui.state::<crate::CalendarSession>().date.set(date);
    ui.state::<crate::CalendarSession>()
        .view
        .set(CalendarView::Day);
    ui.state::<crate::CalendarSession>().selected.set(None);
    ui.state::<crate::CalendarSession>().expanded_day.set(None);
}

pub fn edit_selected(vm: &PhoneViewModel, ui: &Session) {
    let Some(selected) = ui
        .state::<crate::CalendarSession>()
        .selected
        .get_untracked()
    else {
        return;
    };
    let Some(event) = vm
        .data()
        .events
        .iter()
        .find(|event| event.id == selected.event_id)
        .cloned()
    else {
        return;
    };
    if event.repeat == CalendarRepeat::Never {
        ui.state::<crate::CalendarSession>().begin(event, None);
    } else {
        ui.state::<crate::CalendarSession>()
            .scope
            .set(Some(ScopeAction::Edit));
    }
}

pub fn apply_scope(vm: Rc<PhoneViewModel>, ui: Rc<Session>, series: bool) {
    let Some(selected) = ui
        .state::<crate::CalendarSession>()
        .selected
        .get_untracked()
    else {
        return;
    };
    let event = vm
        .data()
        .events
        .iter()
        .find(|event| event.id == selected.event_id)
        .cloned();
    let Some(event) = event else {
        ui.state::<crate::CalendarSession>().reset_transient();
        return;
    };
    match ui.state::<crate::CalendarSession>().scope.get_untracked() {
        Some(ScopeAction::Edit) => ui
            .state::<crate::CalendarSession>()
            .begin(event, if series { None } else { Some(selected.date) }),
        Some(ScopeAction::Delete) => {
            ui.state::<crate::CalendarSession>().scope.set(None);
            let state = ui.clone();
            ui.confirm(
                if series {
                    "delete this appointment series?"
                } else {
                    "delete this appointment?"
                },
                Rc::new(move || {
                    vm.clear_status();
                    if series || event.repeat == CalendarRepeat::Never {
                        vm.delete_event(event.id);
                    } else {
                        vm.dispatch(Action::Calendar(CalendarAction::DeleteOccurrence {
                            id: event.id,
                            date: selected.date.clone(),
                        }));
                    }
                    if vm.status().is_empty() {
                        state.state::<crate::CalendarSession>().selected.set(None);
                    }
                }),
            );
        }
        None => {}
    }
}

pub fn delete_selected(vm: Rc<PhoneViewModel>, ui: Rc<Session>) {
    let Some(selected) = ui
        .state::<crate::CalendarSession>()
        .selected
        .get_untracked()
    else {
        return;
    };
    let repeat = vm
        .data()
        .events
        .iter()
        .find(|event| event.id == selected.event_id)
        .map(|event| event.repeat);
    ui.state::<crate::CalendarSession>()
        .scope
        .set(Some(ScopeAction::Delete));
    if repeat == Some(CalendarRepeat::Never) {
        apply_scope(vm, ui, true);
    }
}

pub fn save(vm: &PhoneViewModel, ui: &Session) {
    let state = &ui.state::<crate::CalendarSession>();
    let Some(target) = state.editor.get_untracked() else {
        return;
    };
    let Some(date) = state.date_value.get_untracked().map(stored_date) else {
        state.error.set("Choose the appointment date.".into());
        return;
    };
    let minute = if state.all_day.get_untracked() {
        0
    } else {
        let Some(time) = state.time_value.get_untracked() else {
            state.error.set("Choose the start time.".into());
            return;
        };
        let minute = time.total_minutes();
        if !(0.0..1440.0).contains(&minute) {
            state.error.set("Choose a valid start time.".into());
            return;
        }
        minute as u32
    };
    let repeat = state.repeat.get_untracked();
    let repeat_until = if repeat != CalendarRepeat::Never && state.repeat_ends.get_untracked() {
        let Some(until) = state.until.get_untracked().map(stored_date) else {
            state.error.set("Choose when repeating ends.".into());
            return;
        };
        Some(until)
    } else {
        None
    };
    let mut excluded_dates = target.original.excluded_dates.clone();
    // Changing the series rule/start creates a new schedule; old exceptions do
    // not silently suppress unrelated dates on the new schedule.
    if repeat != target.original.repeat || date != target.original.date {
        excluded_dates.clear();
    }
    if let Some(until) = &repeat_until {
        excluded_dates.retain(|date| date <= until);
    }
    let event = CalendarEvent {
        id: target.original.id,
        title: state.subject.get_untracked().trim().to_owned(),
        date: date.clone(),
        time: format!("{:02}:{:02}", minute / 60, minute % 60),
        duration_minutes: state.duration.get_untracked(),
        location: state.location.get_untracked().trim().to_owned(),
        details: state.notes.get_untracked(),
        all_day: state.all_day.get_untracked(),
        repeat,
        repeat_until,
        excluded_dates,
        reminder_minutes: state.reminder.get_untracked(),
        status: state.status.get_untracked(),
    };
    if let Err(error) = validate_calendar_event(&event) {
        state.error.set(error);
        return;
    }
    vm.clear_status();
    if let Some(occurrence_date) = target.occurrence_date {
        vm.dispatch(Action::Calendar(CalendarAction::SaveOccurrence {
            event,
            occurrence_date,
        }));
    } else {
        vm.save_event(event);
    }
    let error = vm.status();
    if error.is_empty() {
        state.editor.set(None);
        state.selected.set(None);
        state.error.set(String::new());
        state.date.set(date.clone());
        state.expanded_day.set(Some(date));
    } else {
        state.error.set(error);
    }
}
