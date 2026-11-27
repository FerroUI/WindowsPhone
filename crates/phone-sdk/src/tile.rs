use crate::AppContext;
use crate::session::Session;
use phone_domain::TileSize;
use phone_domain::{Id, Tile, TileTarget};
pub use phone_domain::{TileBadge, TileBadgeGlyph, TileNotification, TileTemplate};
use phone_state::PhoneViewModel;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

/// Optional reactive-owner policy for a newly mounted tile surface. This
/// controls its decorative entrance independently of live content/cadence.
/// The shell uses it for the first mount that replaces a displayed preview;
/// ordinary entry and later content updates retain the default policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileEntrancePolicy {
    pub animate: bool,
}
impl Default for TileEntrancePolicy {
    fn default() -> Self {
        Self { animate: true }
    }
}
impl TileEntrancePolicy {
    pub fn current() -> Self {
        ferroui_view::use_context::<Self>().unwrap_or_default()
    }
}

/// A bounded retained tile surface. System chrome supplies the saved tile label.
#[derive(Clone)]
pub struct AppTileContext {
    pub app: AppContext,
    pub size: TileSize,
    pub width: f64,
    pub height: f64,
    pub path: String,
    /// Transient system cadence; reading it does not persist application data.
    pub now: i64,
    /// False when system settings or power policy pause live tiles.
    pub live: bool,
    /// Respect reduced motion and Start editing before animating tile changes.
    pub animate: bool,
    /// A tile's saved preference; app-owned badge overlays must honor it.
    pub show_badge: bool,
}

#[derive(Default)]
struct TileScopes {
    closed: Cell<bool>,
    generation: Cell<u64>,
    apps: RefCell<BTreeMap<String, u64>>,
}

/// A weak publisher for one registered app and one phone-document epoch.
/// It cannot address another app's tile or continue after restore, app reset,
/// app closing or host shutdown. It does not schedule an OS background agent.
#[derive(Clone)]
pub struct TileUpdater {
    vm: Weak<PhoneViewModel>,
    session: Weak<Session>,
    app: &'static str,
    epoch: u64,
    generation: u64,
    app_generation: u64,
}
impl AppContext {
    pub fn tile_updater(&self) -> Result<TileUpdater, String> {
        let registry = self.session.registry();
        let app = registry
            .by_id(self.id)
            .ok_or("This app is not registered.")?;
        if app.manifest.builtin.map_or_else(
            || !self.vm.extension_enabled(self.id),
            |id| !self.vm.data().installed.contains(&id),
        ) {
            return Err("This app is disabled or unavailable.".into());
        }
        let scope = self.session.state::<TileScopes>();
        if scope.closed.get() {
            return Err("The tile publisher's host is closed.".into());
        }
        let app_generation = *scope.apps.borrow_mut().entry(self.id.into()).or_default();
        Ok(TileUpdater {
            vm: Rc::downgrade(&self.vm),
            session: Rc::downgrade(&self.session),
            app: self.id,
            epoch: self.vm.document_epoch(),
            generation: scope.generation.get(),
            app_generation,
        })
    }
}
impl TileUpdater {
    fn scope(&self) -> Result<(Rc<PhoneViewModel>, Rc<Session>), String> {
        let vm = self.vm.upgrade().ok_or("The phone has closed.")?;
        let ui = self
            .session
            .upgrade()
            .ok_or("The app session has closed.")?;
        let scope = ui.state::<TileScopes>();
        if scope.closed.get()
            || scope.generation.get() != self.generation
            || scope.apps.borrow().get(self.app).copied() != Some(self.app_generation)
            || vm.document_epoch() != self.epoch
        {
            return Err(
                "This tile publisher no longer owns the current app/document session.".into(),
            );
        }
        let registry = ui.registry();
        let app = registry
            .by_id(self.app)
            .ok_or("This app is no longer registered.")?;
        if app.manifest.builtin.map_or_else(
            || !vm.extension_enabled(self.app),
            |id| !vm.data().installed.contains(&id),
        ) {
            return Err("This app is disabled or unavailable.".into());
        }
        Ok((vm, ui))
    }
    fn owns(&self, vm: &PhoneViewModel, ui: &Session, tile: &Tile) -> bool {
        if tile.is_folder() {
            return false;
        }
        if let Some(TileTarget::AppLink(id)) = tile.target {
            vm.data()
                .app_links
                .iter()
                .any(|link| link.id == id && link.app == self.app)
        } else {
            ui.registry()
                .by_id(self.app)
                .is_some_and(|app| app.manifest.builtin == Some(tile.app))
        }
    }
    pub fn tiles(&self) -> Result<Vec<Id>, String> {
        let (vm, ui) = self.scope()?;
        let ids = vm
            .data()
            .all_tiles()
            .filter(|tile| self.owns(&vm, &ui, tile))
            .map(|tile| tile.id)
            .collect();
        Ok(ids)
    }
    pub fn publish(&self, tile: Id, notification: TileNotification) -> Result<(), String> {
        let (vm, ui) = self.scope()?;
        if !vm
            .data()
            .tile(tile)
            .is_some_and(|tile| self.owns(&vm, &ui, tile))
        {
            return Err("This app does not own that pinned tile.".into());
        }
        vm.publish_tile_notification(tile, notification)
    }
    pub fn clear(&self, tile: Id, tag: Option<&str>) -> Result<(), String> {
        let (vm, ui) = self.scope()?;
        if !vm
            .data()
            .tile(tile)
            .is_some_and(|tile| self.owns(&vm, &ui, tile))
        {
            return Err("This app does not own that pinned tile.".into());
        }
        vm.clear_tile_notifications(tile, tag)
    }
}

pub(crate) fn invalidate_app(ui: &Session, app: &str) {
    let scope = ui.state::<TileScopes>();
    if let Some(generation) = scope.apps.borrow_mut().get_mut(app) {
        match generation.checked_add(1) {
            Some(next) => *generation = next,
            None => scope.closed.set(true),
        }
    };
}
pub(crate) fn invalidate_session(ui: &Session, closed: bool) {
    let scope = ui.state::<TileScopes>();
    match scope.generation.get().checked_add(1) {
        Some(next) => scope.generation.set(next),
        None => scope.closed.set(true),
    }
    scope.apps.borrow_mut().clear();
    if closed {
        scope.closed.set(true);
    }
}
