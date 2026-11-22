use crate::PhoneViewModel;
use phone_domain::{AppId, Route};

impl PhoneViewModel {
    pub fn navigate(&self, route: Route) {
        if !self.valid_route(&route) {
            return;
        }
        let current = self.route();
        if current == route {
            return;
        }
        if current.is_overlay() && !route.is_overlay() {
            // Dismissing an Action Center/lock/task-switcher overlay does not leave
            // a spurious overlay entry in the application's Back journal.
            self.history.borrow_mut().pop();
        } else {
            let mut history = self.history.borrow_mut();
            history.push(current.clone());
            if history.len() > 128 {
                history.remove(0);
            }
        }
        self.remember(&current);
        self.remember(&route);
        *self.current_route.borrow_mut() = route.clone();
        match route {
            Route::Conversation(id) => self.mark_conversation_read(id),
            Route::Mail(id) => self.mark_mail(id, true),
            _ => {}
        }
        self.clear_status();
        self.notify();
    }
    pub fn home(&self) {
        self.navigate(Route::Start);
    }
    pub fn back(&self) {
        let current = self.route();
        self.remember(&current);
        let next = loop {
            match self.history.borrow_mut().pop() {
                Some(route) if self.valid_route(&route) => break route,
                Some(_) => continue,
                None => break Route::Start,
            }
        };
        if next != current {
            *self.current_route.borrow_mut() = next;
            self.clear_status();
            self.notify();
        }
    }
    pub fn recent_apps(&self) -> Vec<AppId> {
        self.recent.borrow().iter().map(|(app, _)| *app).collect()
    }
    pub fn resume_app(&self, app: AppId) {
        let route = self
            .recent
            .borrow()
            .iter()
            .find(|(id, _)| *id == app)
            .map(|(_, route)| route.clone())
            .filter(|route| self.valid_route(route))
            .unwrap_or(Route::App(app));
        self.navigate(route);
    }
    pub fn close_app(&self, app: AppId) {
        self.recent.borrow_mut().retain(|(id, _)| *id != app);
        self.history
            .borrow_mut()
            .retain(|route| route.app() != Some(app));
        if self.route().app() == Some(app) {
            *self.current_route.borrow_mut() = Route::Start;
        }
        self.notify();
    }
    fn remember(&self, route: &Route) {
        if !self.valid_route(route) {
            return;
        }
        if let Some(app) = route.app() {
            let mut recent = self.recent.borrow_mut();
            recent.retain(|(id, _)| *id != app);
            recent.insert(0, (app, route.clone()));
            recent.truncate(12);
        }
    }
    /// Deleting a detail preserves the app in recents, but resumes its hub.
    /// Uninstalled apps and deleted details cannot remain in the Back journal.
    pub(crate) fn repair_navigation(&self) {
        self.history
            .borrow_mut()
            .retain(|route| self.valid_route(route));
        self.recent.borrow_mut().retain_mut(|(app, route)| {
            let hub = Route::App(*app);
            if !self.valid_route(&hub) {
                return false;
            }
            if !self.valid_route(route) {
                *route = hub;
            }
            true
        });
    }
    pub(crate) fn valid_route(&self, route: &Route) -> bool {
        let data = self.data();
        if route
            .app()
            .is_some_and(|app| !data.installed.contains(&app))
        {
            return false;
        }
        match route {
            Route::Conversation(id) => data.conversation(*id).is_some(),
            Route::Contact(id) => data.contact(*id).is_some(),
            Route::Mail(id) => data.mail(*id).is_some(),
            Route::Note(id) => data.note(*id).is_some(),
            Route::Folder(id) => data.tile(*id).is_some_and(|t| t.is_folder()),
            _ => true,
        }
    }
}
