use phone_sdk::{AppRegistration, session::Session};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Office, crate::view)
        .with_route_pivot_count(|route, ui| {
            if !matches!(route, phone_domain::Route::App(phone_domain::AppId::Office)) {
                return 0;
            }
            let state = ui.state::<phone_office_support::OfficeSession>();
            if state.show_new.get_untracked() || state.show_find.get_untracked() {
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
    crate::hub::reset_scroll(ui);
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

#[cfg(test)]
mod tests {
    use super::*;
    use phone_domain::{AppId, PhoneData, Route};
    use phone_sdk::AppRegistry;

    #[test]
    fn office_pivots_belong_only_to_the_unobscured_hub_and_back_restores_its_selection() {
        let registry = Rc::new(AppRegistry::new(vec![registration()]).unwrap());
        let ui = Rc::new(Session::new(0, registry.clone()));
        let vm = PhoneViewModel::new(PhoneData::default(), Rc::new(|_| {}));
        vm.navigate(Route::App(AppId::Office));
        let root = vm.route();
        ui.pivot.set(1);
        assert_eq!(registry.pivot_count(&root, &ui), 2);
        assert!(!registry.can_swipe(&Route::Document(1), &ui));
        let state = ui.state::<phone_office_support::OfficeSession>();
        state.show_find.set(true);
        ui.query.set("a document".into());
        assert_eq!(registry.pivot_count(&root, &ui), 0);
        assert!(!registry.can_swipe(&root, &ui));
        state.show_new.set(true);
        assert_eq!(registry.pivot_count(&root, &ui), 0);
        assert!(registry.back(&vm, &ui));
        assert!(!state.show_new.get_untracked());
        assert!(state.show_find.get_untracked());
        assert!(!registry.can_swipe(&root, &ui));
        assert!(registry.back(&vm, &ui));
        assert!(!state.show_find.get_untracked());
        assert!(ui.query.get_untracked().is_empty());
        assert_eq!(ui.pivot.get_untracked(), 1);
        assert_eq!(registry.pivot_count(&root, &ui), 2);
        assert!(!registry.back(&vm, &ui));
        state.show_new.set(true);
        assert!(!registry.can_swipe(&root, &ui));
    }
}
