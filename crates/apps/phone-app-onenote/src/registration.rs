use crate::state::{NoteSession, Pane};
use phone_domain::{AppId, Route};
use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;
pub fn registration() -> AppRegistration {
    AppRegistration::builtin(AppId::OneNote, crate::view)
        .with_back(back)
        .with_deactivate(|ui| ui.state::<NoteSession>().moving.set(None))
        .with_reset(|ui| {
            ui.state::<NoteSession>().pane.set(Pane::Recent);
            ui.state::<NoteSession>().moving.set(None);
        })
        .with_overlay(|ui| ui.state::<NoteSession>().moving.get_untracked().is_some())
}
fn back(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    let state = ui.state::<NoteSession>();
    if state.moving.get_untracked().is_some() {
        state.moving.set(None);
        return true;
    }
    if vm.route() != Route::App(AppId::OneNote) {
        return false;
    }
    let next = match state.pane.get_untracked() {
        Pane::Pages(id) => crate::notebooks::read(vm)
            .ok()
            .and_then(|library| {
                library
                    .section(id)
                    .map(|(notebook, _)| Pane::Sections(notebook.id))
            })
            .unwrap_or(Pane::Notebooks),
        Pane::Sections(_) => Pane::Notebooks,
        _ => return false,
    };
    state.pane.set(next);
    ui.query.set(String::new());
    true
}
