use crate::state::MapSession;
use phone_sdk::{session::Session, AppRegistration};
use std::rc::Rc;
pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Maps, crate::view)
        .with_pivot_count(|ui| {
            let state = ui.state::<MapSession>();
            if state.selection.get_untracked().is_some() || state.directions.get_untracked() {
                0
            } else {
                3
            }
        })
        .with_back(|_, ui| {
            let state = ui.state::<MapSession>();
            if state.directions.get_untracked() {
                state.directions.set(false);
                true
            } else if state.selection.get_untracked().is_some() {
                state.selection.set(None);
                true
            } else {
                false
            }
        })
        .with_overlay(|ui| {
            let state = ui.state::<MapSession>();
            state.selection.get_untracked().is_some() || state.directions.get_untracked()
        })
        .with_deactivate(cancel)
        .with_reset(|ui| {
            cancel(ui);
            let state = ui.state::<MapSession>();
            state.selection.set(None);
            state.directions.set(false);
            state.results.set(Vec::new());
            state.origin.set(String::new());
            state.destination.set(String::new());
        })
}
fn cancel(ui: &Rc<Session>) {
    let state = ui.state::<MapSession>();
    state.generation.set(state.generation.get().wrapping_add(1));
    state.busy.set(false);
}
