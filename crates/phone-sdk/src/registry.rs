use crate::{session::Session, AppLifecycle, AppManifest, AppRegistration};
use ferroui_view::prelude::Element;
use phone_domain::{AppId, Route};
use phone_state::PhoneViewModel;
use std::{collections::BTreeMap, rc::Rc};

mod events;

/// Immutable, validated composition. Lookup is logarithmic and registration
/// order cannot silently replace another application's identity or built-in.
pub struct AppRegistry {
    entries: Vec<AppRegistration>,
    ids: BTreeMap<&'static str, usize>,
    builtins: BTreeMap<AppId, usize>,
}
impl AppRegistry {
    pub fn new(entries: Vec<AppRegistration>) -> Result<Self, String> {
        if entries.len() > phone_domain::MAX_APP_NAMESPACES {
            return Err("Too many registered apps.".into());
        }
        let mut ids = BTreeMap::new();
        let mut builtins = BTreeMap::new();
        for (index, item) in entries.iter().enumerate() {
            let manifest = item.manifest;
            phone_domain::validate_app_key(manifest.id)?;
            if manifest.title.trim().is_empty()
                || manifest.title.chars().count() > 80
                || manifest.title.chars().any(char::is_control)
            {
                return Err(format!("Invalid title for {}.", manifest.id));
            }
            if ids.insert(manifest.id, index).is_some() {
                return Err(format!("Duplicate application: {}.", manifest.id));
            }
            if let Some(app) = manifest.builtin {
                if builtins.insert(app, index).is_some() {
                    return Err(format!("Duplicate built-in: {}.", app.label()));
                }
            }
        }
        Ok(Self {
            entries,
            ids,
            builtins,
        })
    }
    pub fn activate(
        &self,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        id: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let app = self
            .by_id(id)
            .ok_or("This application is not registered.")?;
        if app.manifest.builtin.map_or_else(
            || !vm.extension_enabled(id),
            |id| !vm.data().installed.contains(&id),
        ) {
            return Err("This application is unavailable or disabled.".into());
        }
        if self
            .for_route(&vm.route())
            .is_none_or(|active| active.manifest.id != id)
        {
            return Err("Navigate to the application before delivering activation data.".into());
        }
        phone_domain::validate_app_value(&payload)?;
        let callback = app
            .activate
            .ok_or("This application does not accept data activation.")?;
        callback(vm, ui, payload)
    }
    pub fn tile(
        &self,
        id: &str,
        vm: &Rc<PhoneViewModel>,
        ui: &Rc<Session>,
        size: phone_domain::TileSize,
        width: f64,
        height: f64,
        path: String,
        now: i64,
    ) -> Option<Element> {
        let app = self.by_id(id)?;
        app.tile.map(|render| {
            render(crate::AppTileContext {
                app: app.context(vm, ui),
                size,
                width,
                height,
                path,
                now,
                live: vm.live_tiles_enabled(),
                animate: vm.live_tiles_enabled()
                    && vm.motion_enabled()
                    && ui.edit_tile.get().is_none(),
            })
        })
    }
    pub fn by_id(&self, id: &str) -> Option<&AppRegistration> {
        self.ids.get(id).map(|index| &self.entries[*index])
    }
    pub fn uses_backspace_input(&self, route: &Route) -> bool {
        self.for_route(route).is_some_and(|app| app.backspace_input)
    }
    pub fn for_route(&self, route: &Route) -> Option<&AppRegistration> {
        if let Route::Extension { app, .. } = route {
            return self
                .by_id(app)
                .filter(|item| item.manifest.builtin.is_none());
        }
        route
            .app()
            .and_then(|id| self.builtins.get(&id))
            .map(|index| &self.entries[*index])
    }
    pub fn manifests(&self) -> Vec<AppManifest> {
        self.entries.iter().map(|item| item.manifest).collect()
    }
    pub fn view(&self, vm: Rc<PhoneViewModel>, ui: Rc<Session>) -> Element {
        self.for_route(&vm.route())
            .map(|app| (app.render)(app.context(&vm, &ui)))
            .unwrap_or_else(|| {
                crate::common::empty(
                    "app unavailable",
                    "This application is not registered in this build.",
                )
            })
    }
    pub fn back(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>) -> bool {
        self.for_route(&vm.route())
            .and_then(|app| app.back)
            .is_some_and(|callback| callback(vm, ui))
    }
    pub fn pivot_count(&self, route: &Route, ui: &Session) -> usize {
        self.for_route(route)
            .and_then(|app| app.pivot_count)
            .map(|callback| callback(ui))
            .unwrap_or(0)
    }
    pub fn can_swipe(&self, route: &Route, ui: &Session) -> bool {
        self.for_route(route)
            .and_then(|app| app.can_swipe)
            .is_some_and(|callback| callback(ui))
            || self.pivot_count(route, ui) > 0
    }
    pub fn swipe(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, delta: f64) -> bool {
        self.for_route(&vm.route())
            .and_then(|app| app.swipe)
            .is_some_and(|callback| callback(vm, ui, delta))
    }
    pub fn has_overlay(&self, route: &Route, ui: &Session) -> bool {
        self.for_route(route)
            .and_then(|app| app.overlay)
            .is_some_and(|callback| callback(ui))
    }
    pub fn tick(&self, vm: &Rc<PhoneViewModel>, ui: &Rc<Session>, now: i64) {
        if let Some(callback) = self.for_route(&vm.route()).and_then(|app| app.tick) {
            callback(vm, ui, now);
        }
    }
}
