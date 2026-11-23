use crate::PhoneViewModel;
use phone_domain::{AppId, Route};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecentApp {
    Builtin(AppId),
    Extension(String),
}

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
    pub fn recent_entries(&self) -> Vec<RecentApp> {
        self.recent_order.borrow().clone()
    }
    pub fn resume_extension(&self, app: &str) {
        let route = self
            .extension_recent
            .borrow()
            .iter()
            .find(|(id, _)| id == app)
            .map(|(_, route)| route.clone())
            .filter(|route| self.valid_route(route))
            .unwrap_or(Route::Extension {
                app: app.into(),
                path: String::new(),
            });
        self.navigate(route);
    }
    pub fn close_extension(&self, app: &str) {
        self.extension_recent
            .borrow_mut()
            .retain(|(id, _)| id != app);
        self.recent_order
            .borrow_mut()
            .retain(|entry| !matches!(entry,RecentApp::Extension(id) if id==app));
        self.history
            .borrow_mut()
            .retain(|route| !matches!(route,Route::Extension{app:id,..} if id==app));
        if matches!(self.route(),Route::Extension{app:id,..} if id==app) {
            *self.current_route.borrow_mut() = Route::Start;
        }
        self.notify();
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
        self.recent_order
            .borrow_mut()
            .retain(|entry| *entry != RecentApp::Builtin(app));
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
        if let Route::Extension { app, .. } = route {
            let mut recent = self.extension_recent.borrow_mut();
            recent.retain(|(id, _)| id != app);
            recent.insert(0, (app.clone(), route.clone()));
            recent.truncate(12);
            self.order_recent(RecentApp::Extension(app.clone()));
        }
        if let Some(app) = route.app() {
            self.order_recent(RecentApp::Builtin(app));
            let mut recent = self.recent.borrow_mut();
            recent.retain(|(id, _)| *id != app);
            recent.insert(0, (app, route.clone()));
            recent.truncate(12);
        }
    }
    fn order_recent(&self, app: RecentApp) {
        let mut order = self.recent_order.borrow_mut();
        order.retain(|entry| *entry != app);
        order.insert(0, app);
        order.truncate(12);
    }
    /// Deleting a detail preserves the app in recents, but resumes its hub.
    /// Uninstalled apps and deleted details cannot remain in the Back journal.
    pub(crate) fn repair_navigation(&self) {
        self.extension_recent
            .borrow_mut()
            .retain(|(_, route)| self.valid_route(route));
        self.recent_order.borrow_mut().retain(|entry| match entry {
            RecentApp::Builtin(app) => self.valid_route(&Route::App(*app)),
            RecentApp::Extension(id) => self.extension_enabled(id),
        });
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
            Route::Extension { app, path } => {
                self.registered_apps.borrow().contains(app)
                    && !data.disabled_extensions.contains(app)
                    && phone_domain::validate_app_path(path).is_ok()
            }
            Route::Conversation(id) => data.conversation(*id).is_some(),
            Route::Contact(id) => data.contact(*id).is_some(),
            Route::Mail(id) => data.mail(*id).is_some(),
            Route::Note(id) => data.note(*id).is_some(),
            Route::Document(id) => data.document(*id).is_some(),
            Route::FileFolder(Some(id)) => {
                data.document_folders.iter().any(|folder| folder.id == *id)
            }
            Route::Folder(id) => data.tile(*id).is_some_and(|t| t.is_folder()),
            _ => true,
        }
    }
}
