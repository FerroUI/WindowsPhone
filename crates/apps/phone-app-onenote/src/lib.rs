//! Local notebooks, sections and pages over the phone SDK's persisted document.
use ferroui_view::prelude::*;
use phone_domain::Route;
use phone_sdk::{common::*, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;
mod body;
mod checklist;
mod commands;
mod detail;
mod import;
mod notebooks;
mod registration;
mod state;
pub use registration::registration;

pub fn view(vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
    let library = match notebooks::read(&vm) {
        Ok(library) => library,
        Err(error) => {
            return page(
                "OneNote",
                empty("Notebook data unavailable", &error),
                Element::Empty,
            )
        }
    };
    if let Some(id) = ui.state::<state::NoteSession>().moving.get() {
        return notebooks::view::move_page(vm, ui, library, id);
    }
    if let Route::Note(id) = vm.route() {
        if let Some(note) = vm.data().note(id).cloned() {
            return detail::view(vm.clone(), ui, note, &library);
        }
    }
    notebooks::view::view(vm, ui, library)
}
