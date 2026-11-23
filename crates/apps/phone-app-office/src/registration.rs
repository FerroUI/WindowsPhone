use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Office, crate::view)
        .with_pivot_count(|ui| {
            if ui
                .state::<phone_office_support::OfficeSession>()
                .show_new
                .get_untracked()
            {
                0
            } else {
                2
            }
        })
        .with_back(back)
        .with_route_changed(|_, ui, _, _| clear(ui))
        .with_deactivate(clear)
        .with_reset(reset)
        .with_overlay(|ui| {
            let state = ui.state::<phone_office_support::OfficeSession>();
            state.show_new.get_untracked() || state.show_find.get_untracked()
        })
}

fn clear(ui: &Rc<Session>) {
    let state = ui.state::<phone_office_support::OfficeSession>();
    state.show_new.set(false);
    state.show_find.set(false);
    state.formatting.set(false);
}
fn reset(ui: &Rc<Session>) {
    clear(ui);
    let state = ui.state::<phone_office_support::OfficeSession>();
    state.paragraph.set(0);
    state.sheet.set(0);
    state.excel_selection.set(None);
    state.row_page.set(0);
    state.col_page.set(0);
    state.slide.set(0);
}
fn back(_: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
    let state = ui.state::<phone_office_support::OfficeSession>();
    if state.show_new.get_untracked() {
        state.show_new.set(false);
        true
    } else if state.show_find.get_untracked() {
        state.show_find.set(false);
        ui.query.set(String::new());
        true
    } else {
        false
    }
}
