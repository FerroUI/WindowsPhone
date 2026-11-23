use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Files, crate::view)
        .with_back(back)
        .with_route_changed(route_changed)
        .with_deactivate(clear)
        .with_reset(clear)
        .with_overlay(|ui| {
            ui.state::<crate::FilesSession>()
                .properties
                .get_untracked()
                .is_some()
        })
}

fn clear(ui: &Rc<Session>) {
    let state = ui.state::<crate::FilesSession>();
    state.clear_properties();
    state.folder_menu.set(None);
    state.selecting.set(false);
    state.selection.set(Default::default());
    state.searching.set(false);
    state.transfer.set(None);
}
fn route_changed(
    _: &Rc<PhoneViewModel>,
    ui: &Rc<Session>,
    _: &phone_domain::Route,
    next: &phone_domain::Route,
) {
    ui.state::<crate::FilesSession>().clear_properties();
    if !matches!(
        next,
        phone_domain::Route::FileFolder(_) | phone_domain::Route::App(phone_domain::AppId::Files)
    ) {
        clear(ui);
    }
}
fn back(_: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    let state = ui.state::<crate::FilesSession>();
    if state.dismiss_properties() {
        true
    } else if state.folder_menu.get_untracked().is_some() {
        state.folder_menu.set(None);
        true
    } else if state.selecting.get_untracked() {
        state.selecting.set(false);
        state.selection.set(Default::default());
        true
    } else if state.searching.get_untracked() {
        state.searching.set(false);
        ui.query.set(String::new());
        true
    } else if state.transfer.get_untracked().is_some() {
        state.transfer.set(None);
        true
    } else {
        false
    }
}
