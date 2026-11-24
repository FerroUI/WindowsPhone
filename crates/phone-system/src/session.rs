use crate::{app_list::AppListState, hardware::HardwareState, previews::PreviewCache};
use phone_alarm_service::AlarmState;
use phone_domain::Route;
pub use phone_sdk::session::Session;
use phone_state::PhoneViewModel;
use std::rc::Rc;

pub trait SystemSessionExt {
    fn dismiss(self: &Rc<Self>, vm: &Rc<PhoneViewModel>) -> bool;
    fn route_changed(self: &Rc<Self>, vm: &Rc<PhoneViewModel>, route: &Route);
    fn clear_overlays(self: &Rc<Self>);
    fn reconcile_document(self: &Rc<Self>, vm: &Rc<PhoneViewModel>);
    fn tick_live_tiles(&self, now: i64);
    fn preview_obscured(&self, route: &Route) -> bool;
}
impl SystemSessionExt for Session {
    fn dismiss(self: &Rc<Self>, vm: &Rc<PhoneViewModel>) -> bool {
        let hardware = self.state::<HardwareState>();
        if hardware.is_open() {
            hardware.close();
        } else if self.dialog.get_untracked().is_some() {
            self.dialog.set(None);
        } else if self.notification_open.get_untracked() {
            self.notification_open.set(false);
            self.notification_peek.set(false);
        } else if self.switcher_open.get_untracked() {
            self.switcher_open.set(false);
        } else if self.menu.get_untracked() {
            self.menu.set(false);
        } else if crate::app_list::dismiss_native(self) {
        } else if self.registry().back(vm, self) {
        } else if self.alphabet.get_untracked() {
            self.alphabet.set(false);
        } else if self.edit_tile.get_untracked().is_some() {
            self.edit_tile.set(None);
        } else if self.folder.get_untracked().is_some() {
            self.folder.set(None);
        } else if self.selected.get_untracked().is_some() {
            self.selected.set(None);
        } else {
            return false;
        }
        true
    }
    fn route_changed(self: &Rc<Self>, vm: &Rc<PhoneViewModel>, route: &Route) {
        let previous = self.last_route.borrow().clone();
        if previous == *route {
            return;
        }
        if matches!(route, Route::Start | Route::Folder(_)) {
            self.live_clock.set(vm.now());
        }
        if let Some(key) = route_key(&previous) {
            self.app_pivots
                .borrow_mut()
                .insert(key.clone(), self.pivot.get_untracked());
            self.app_selections
                .borrow_mut()
                .insert(key, self.selected.get_untracked());
        }
        self.pivot.set(
            route_key(route)
                .and_then(|key| self.app_pivots.borrow().get(&key).copied())
                .unwrap_or(0),
        );
        self.selected.set(
            route_key(route)
                .and_then(|key| self.app_selections.borrow().get(&key).copied())
                .flatten(),
        );
        self.query.set(String::new());
        self.menu.set(false);
        self.alphabet.set(false);
        self.dialog.set(None);
        self.edit_tile.set(None);
        self.state::<AlarmState>().editor.close();
        self.registry().route_changed(vm, self, &previous, route);
        *self.last_route.borrow_mut() = route.clone();
    }
    fn clear_overlays(self: &Rc<Self>) {
        if phone_controls::has_open_phone_picker() {
            self.state::<crate::page_motion::NavigationMotion>().cancel();
            phone_controls::close_phone_pickers();
        }
        crate::app_list::dismiss_native(self);
        self.state::<HardwareState>().close();
        self.state::<AlarmState>().editor.close();
        let previous = self.last_route.borrow().clone();
        self.registry().deactivate(&previous, self);
        self.dialog.set(None);
        self.menu.set(false);
        self.notification_open.set(false);
        self.notification_peek.set(false);
        self.switcher_open.set(false);
        self.alphabet.set(false);
        self.edit_tile.set(None);
        self.folder.set(None);
    }
    fn reconcile_document(self: &Rc<Self>, vm: &Rc<PhoneViewModel>) {
        if self.document_epoch.replace(vm.document_epoch()) == vm.document_epoch() {
            return;
        }
        self.clear_activation();
        self.live_clock.set(vm.now());
        self.state::<AlarmState>().clear();
        self.state::<AppListState>().reset();
        crate::gestures::cancel();
        self.clear_overlays();
        self.state::<PreviewCache>().clear();
        self.state::<crate::page_motion::NavigationMotion>().reset();
        self.registry().reset(vm, self);
        self.app_pivots.borrow_mut().clear();
        self.app_selections.borrow_mut().clear();
        *self.last_route.borrow_mut() = Route::Start;
        self.query.set(String::new());
        self.entry.set(String::new());
        self.selected.set(None);
        self.pivot.set(0);
        for field in &self.fields {
            field.set(String::new());
        }
    }
    fn tick_live_tiles(&self, now: i64) {
        crate::start::tick_notifications(self, now);
        if self.live_clock_enabled.get()
            && matches!(*self.last_route.borrow(), Route::Start | Route::Folder(_))
            && !self.preview_obscured(&Route::Start)
            && self.live_clock.get_untracked().unsigned_abs() / 4 != now.unsigned_abs() / 4
        {
            self.live_clock.set(now);
        }
    }
    fn preview_obscured(&self, route: &Route) -> bool {
        route.is_overlay()
            || crate::app_list::has_native_overlay(self)
            || self.state::<AlarmState>().active.get_untracked().is_some()
            || self.state::<HardwareState>().is_open()
            || self.notification_open.get_untracked()
            || self.switcher_open.get_untracked()
            || self.locked.get_untracked()
            || self.dialog.get_untracked().is_some()
            || self.menu.get_untracked()
            || self.alphabet.get_untracked()
            || self.edit_tile.get_untracked().is_some()
            || self.folder.get_untracked().is_some()
            || self.registry().has_overlay(route, self)
    }
}
fn route_key(route: &Route) -> Option<String> {
    match route {
        Route::App(app) => Some(app.label().into()),
        Route::Extension { app, .. } => Some(app.clone()),
        _ => None,
    }
}
