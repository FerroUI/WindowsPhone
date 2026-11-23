use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Calendar, crate::view)
        .with_activation(|_, ui, payload| {
            let occurrence: phone_domain::CalendarOccurrence = serde_json::from_value(payload)
                .map_err(|error| format!("Invalid Calendar activation: {error}"))?;
            ui.state::<crate::CalendarSession>()
                .open_occurrence(occurrence);
            Ok(())
        })
        .with_back(|_, ui| ui.state::<crate::CalendarSession>().dismiss())
        .with_route_changed(|_, ui, _, _| ui.state::<crate::CalendarSession>().reset_transient())
        .with_deactivate(|ui| ui.state::<crate::CalendarSession>().reset_transient())
        .with_reset(|ui| ui.state::<crate::CalendarSession>().reset())
        .with_can_swipe(crate::can_swipe)
        .with_swipe(swipe)
        .with_overlay(overlay)
}

fn swipe(vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, delta: f64) -> bool {
    if !crate::can_swipe(ui) {
        return false;
    }
    crate::move_period(vm, ui, if delta < 0.0 { 1 } else { -1 });
    true
}
fn overlay(ui: &Session) -> bool {
    let state = ui.state::<crate::CalendarSession>();
    state.editor.get_untracked().is_some()
        || state.picker.get_untracked().is_some()
        || state.scope.get_untracked().is_some()
}
