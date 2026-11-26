use phone_sdk::{session::Session, AppRegistration};
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub fn registration() -> AppRegistration {
    AppRegistration::builtin(phone_domain::AppId::Calendar, crate::view)
        .with_activation(crate::tasks::activate)
        .with_launch_path(crate::tasks::launch_path)
        .with_reminders(crate::tasks::reminders)
        .with_back(|vm, ui| {
            crate::tasks::back(vm, ui) || ui.state::<crate::CalendarSession>().dismiss()
        })
        .with_route_changed(|_, ui, _, _| {
            ui.state::<crate::CalendarSession>().reset_transient();
            ui.state::<crate::tasks::State>().close_overlays();
        })
        .with_deactivate(|ui| {
            ui.state::<crate::CalendarSession>().reset_transient();
            // Retain task drafts across app switching and privacy takeover.
            // Popups/menus are ephemeral; the editor is a private app surface.
            ui.state::<crate::tasks::State>().close_overlays();
        })
        .with_reset(|ui| {
            ui.state::<crate::CalendarSession>().reset();
            ui.state::<crate::tasks::State>().reset();
        })
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
    let tasks = ui.state::<crate::tasks::State>();
    let state = ui.state::<crate::CalendarSession>();
    tasks.editor.get_untracked().is_some()
        || tasks.choice.get_untracked().is_some()
        || tasks.context.get_untracked().is_some()
        || state.editor.get_untracked().is_some()
        || state.picker.get_untracked().is_some()
        || state.scope.get_untracked().is_some()
}
