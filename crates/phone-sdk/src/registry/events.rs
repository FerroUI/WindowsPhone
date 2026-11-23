use super::*;

impl AppRegistry {
    /// Deliver host observations independently of the foreground route. Each
    /// callback is checked again: a previous subscriber can disable an app,
    /// replace the document, or synchronously switch the active player.
    pub fn media_event(
        &self,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        event: &crate::media::PlaybackEvent,
    ) {
        for app in &self.entries {
            let Some(callback) = app.media_event else {
                continue;
            };
            if !app
                .manifest
                .capabilities
                .contains(&crate::Capability::Media)
                && !app.manifest.capabilities.contains(&crate::Capability::All)
            {
                continue;
            }
            if app.manifest.builtin.map_or_else(
                || !vm.extension_enabled(app.manifest.id),
                |id| !vm.data().installed.contains(&id),
            ) {
                continue;
            }
            if !event.snapshot().is_current(vm, ui) {
                break;
            }
            callback(app.context(vm, ui), event);
        }
    }
    pub fn route_changed(
        &self,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        previous: &Route,
        next: &Route,
    ) {
        let before = self.for_route(previous);
        let after = self.for_route(next);
        let changed = before.map(|app| app.manifest.id) != after.map(|app| app.manifest.id);
        if let Some(app) = before {
            if let Some(callback) = app.route_changed {
                callback(vm, ui, previous, next);
            }
            if changed {
                if let Some(callback) = app.deactivate {
                    callback(ui);
                }
                if let Some(callback) = app.lifecycle {
                    callback(app.context(vm, ui), AppLifecycle::Deactivated);
                }
            }
        }
        if let Some(app) = after {
            if changed {
                if let Some(callback) = app.route_changed {
                    callback(vm, ui, previous, next);
                }
                if let Some(callback) = app.lifecycle {
                    callback(app.context(vm, ui), AppLifecycle::Activated);
                }
            }
        }
    }
    /// Notify an app that its instance is closing. The shell removes navigation
    /// entries; an app decides whether Closing should preserve or reset drafts.
    pub fn close(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: &str) {
        crate::tasks::cancel_for_app(ui, id);
        crate::tile::invalidate_app(ui, id);
        let _ = ui.take_activation(id);
        if let Some(app) = self.by_id(id) {
            if let Some(callback) = app.lifecycle {
                callback(app.context(vm, ui), AppLifecycle::Closing);
            }
        }
    }
    /// Close all app resources when the embedding host goes away. Drafts remain
    /// persistent; SDK tasks and pending activations cannot outlive the host.
    pub fn shutdown(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
        crate::tasks::cancel_for_session(ui);
        ui.clear_activation();
        for app in &self.entries {
            if let Some(callback) = app.lifecycle {
                callback(app.context(vm, ui), AppLifecycle::Closing);
            }
        }
        crate::tile::invalidate_session(ui, true);
    }
    /// Reset one app's transient state after its namespace has been cleared.
    pub fn reset_app(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, id: &str) {
        crate::tasks::cancel_for_app(ui, id);
        crate::tile::invalidate_app(ui, id);
        let _ = ui.take_activation(id);
        if let Some(app) = self.by_id(id) {
            if let Some(callback) = app.reset {
                callback(ui);
            }
            if let Some(callback) = app.lifecycle {
                callback(app.context(vm, ui), AppLifecycle::Reset);
            }
        }
    }
    pub fn reset(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) {
        crate::tasks::cancel_for_session(ui);
        crate::tile::invalidate_session(ui, false);
        ui.clear_activation();
        for app in &self.entries {
            if let Some(callback) = app.reset {
                callback(ui);
            }
            if let Some(callback) = app.lifecycle {
                callback(app.context(vm, ui), AppLifecycle::Reset);
            }
        }
    }
    pub fn deactivate(&self, route: &Route, ui: &Rc<Session>) {
        if let Some(callback) = self.for_route(route).and_then(|app| app.deactivate) {
            callback(ui);
        }
    }
    pub fn lifecycle(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, event: AppLifecycle) {
        if let Some(app) = self.for_route(&vm.route()) {
            if let Some(callback) = app.lifecycle {
                callback(app.context(vm, ui), event);
            }
        }
    }
    pub fn host_navigation(&self, vm: &PhoneViewModel, ui: &Session, url: &str) {
        for app in &self.entries {
            if let Some(callback) = app.host_navigation {
                callback(vm, ui, url);
            }
        }
    }
    pub fn host_weather(&self, ui: &Session, place: String, temperature: f64, condition: String) {
        for app in &self.entries {
            if let Some(callback) = app.host_weather {
                callback(ui, place.clone(), temperature, condition.clone());
            }
        }
    }
}
