use crate::PhoneViewModel;
use phone_domain::{validate_app_key, MAX_APP_NAMESPACES, MAX_BACKUP_BYTES};

impl PhoneViewModel {
    pub fn app_data(&self, id: &str) -> Option<serde_json::Value> {
        self.data().app_data.get(id).cloned()
    }
    pub fn set_app_data(&self, id: &str, value: Option<serde_json::Value>) -> Result<(), String> {
        validate_app_key(id)?;
        if self.applying.get() {
            return Err("An app data transaction is already active.".into());
        }
        if let Some(value) = value.as_ref() {
            phone_domain::validate_app_value(value)?;
        }
        let mut candidate = self.document.borrow().clone();
        match value {
            Some(value) => {
                candidate.app_data.insert(id.into(), value);
            }
            None => {
                candidate.app_data.remove(id);
            }
        }
        if candidate.app_data.len() > MAX_APP_NAMESPACES {
            return Err("Too many app data namespaces.".into());
        }
        if candidate.to_json()?.len() > MAX_BACKUP_BYTES {
            return Err("The phone document is full.".into());
        }
        if *self.document.borrow() == candidate {
            return Ok(());
        }
        // The existing save callback receives one stable snapshot with no live
        // document borrow held. Nested app-data writes are rejected.
        self.applying.set(true);
        *self.document.borrow_mut() = candidate;
        self.persist();
        self.notify();
        self.applying.set(false);
        // Drain the existing queue in its original order. Popping then pushing
        // an action through apply(Some(...)) would move it behind later actions.
        self.apply(None);
        Ok(())
    }
    pub fn extension_enabled(&self, id: &str) -> bool {
        self.registered_apps.borrow().contains(id) && !self.data().disabled_extensions.contains(id)
    }
    pub fn set_extension_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        if !self.registered_apps.borrow().contains(id) {
            return Err("This application is not registered in this build.".into());
        }
        if self.applying.get() {
            return Err("An app transaction is already active.".into());
        }
        self.dispatch(phone_domain::Action::SetExtensionEnabled {
            app: id.into(),
            enabled,
        });
        if self.extension_enabled(id) == enabled {
            Ok(())
        } else {
            Err(self.status())
        }
    }
    pub fn pin_app(&self, app: &str, path: &str, title: &str) -> Result<(), String> {
        phone_domain::validate_app_link(app, path, title)?;
        if !self.extension_enabled(app) {
            return Err("This application is unavailable or disabled.".into());
        }
        if self.applying.get() {
            return Err("An app transaction is already active.".into());
        }
        self.dispatch(phone_domain::Action::PinApp {
            app: app.into(),
            path: path.into(),
            title: title.into(),
        });
        if self
            .data()
            .app_links
            .iter()
            .any(|link| link.app == app && link.path == path)
        {
            Ok(())
        } else {
            Err(self.status())
        }
    }
    pub fn set_registered_apps(&self, ids: impl IntoIterator<Item = String>) {
        *self.registered_apps.borrow_mut() = ids
            .into_iter()
            .filter(|id| validate_app_key(id).is_ok())
            .collect();
        self.repair_navigation();
        if !self.valid_route(&self.route()) {
            *self.current_route.borrow_mut() = phone_domain::Route::Start;
        }
    }
    /// Builtin identity bindings admit secondary tiles without turning builtins
    /// into disableable extension applications or extension navigation routes.
    pub fn set_registered_builtins(
        &self,
        apps: impl IntoIterator<Item = (String, phone_domain::AppId)>,
    ) {
        *self.registered_builtins.borrow_mut() = apps
            .into_iter()
            .filter(|(id, _)| validate_app_key(id).is_ok())
            .collect();
    }
}
