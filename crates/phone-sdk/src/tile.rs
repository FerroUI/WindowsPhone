use crate::AppContext;
use phone_domain::TileSize;

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
}
