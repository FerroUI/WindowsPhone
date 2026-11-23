use crate::{AppId, Id};
use serde::{Deserialize, Serialize};

mod presentation;
pub use presentation::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileSize {
    Small,
    #[default]
    Medium,
    Wide,
}
impl TileSize {
    pub fn cells(self) -> (usize, usize) {
        match self {
            Self::Small => (1, 1),
            Self::Medium => (2, 2),
            Self::Wide => (4, 2),
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Small => Self::Medium,
            Self::Medium => Self::Wide,
            Self::Wide => Self::Small,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub id: Id,
    pub app: AppId,
    pub size: TileSize,
    pub label: String,
    #[serde(default)]
    pub children: Vec<AppId>,
    /// Canonical child records; the legacy app list is retained for v1 backup compatibility.
    #[serde(default)]
    pub folder_tiles: Vec<Tile>,
    #[serde(default)]
    pub target: Option<TileTarget>,
    /// User choices are separate from the app's current title and notifications.
    #[serde(default)]
    pub appearance: TileAppearance,
    /// At most five bounded cards, including scheduled cards. Expiry is evaluated
    /// against the visible host clock; rendering never writes the phone document.
    #[serde(default)]
    pub notifications: Vec<TileNotification>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileTarget {
    AppLink(Id),
    Contact(Id),
    Note(Id),
    Document(Id),
    Bookmark(Id),
}
impl TileTarget {
    pub fn app(self) -> AppId {
        match self {
            // The v1 app enum remains a presentation fallback; AppLink owns routing.
            Self::AppLink(_) => AppId::Store,
            Self::Contact(_) => AppId::People,
            Self::Note(_) => AppId::OneNote,
            Self::Document(_) => AppId::Office,
            Self::Bookmark(_) => AppId::InternetExplorer,
        }
    }
}
impl Tile {
    pub fn new(id: Id, app: AppId, size: TileSize) -> Self {
        Self {
            id,
            app,
            size,
            label: app.label().into(),
            children: Vec::new(),
            folder_tiles: Vec::new(),
            target: None,
            appearance: TileAppearance::default(),
            notifications: Vec::new(),
        }
    }
    pub fn is_folder(&self) -> bool {
        !self.folder_tiles.is_empty() || !self.children.is_empty()
    }
    pub fn secondary(id: Id, target: TileTarget, label: String) -> Self {
        let mut tile = Self::new(id, target.app(), TileSize::Medium);
        tile.target = Some(target);
        tile.label = label;
        tile
    }
    pub fn sync_children(&mut self) {
        self.children = self.folder_tiles.iter().map(|tile| tile.app).collect();
    }
    pub fn display_label(&self) -> &str {
        self.appearance.label.as_deref().unwrap_or(&self.label)
    }
    /// Newest eligible notification wins when rotation is paused. Future and
    /// expired cards never become visible, including under reduced motion.
    pub fn active_notification(&self, now: i64, rotate: bool) -> Option<&TileNotification> {
        if !self.appearance.live || self.is_folder() {
            return None;
        }
        let active = self
            .notifications
            .iter()
            .rev()
            .filter(|card| card.active_at(now))
            .collect::<Vec<_>>();
        let newest = *active.first()?;
        let index = if rotate && self.appearance.queue {
            (now.saturating_sub(newest.starts_at).max(0) as u64 / TILE_CARD_SECONDS)
                % active.len() as u64
        } else {
            0
        };
        active.get(index as usize).copied()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePlacement {
    pub id: Id,
    pub column: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}

/// Stable first-fit packing on half-tile cells. Every tile is placed exactly once;
/// Compact row masks avoid repeated intersection tests with placed rectangles.
pub fn pack_tiles(tiles: &[Tile], columns: usize) -> Vec<TilePlacement> {
    let columns = columns.clamp(4, 6);
    let mut rows: Vec<u8> = Vec::new();
    let mut placements = Vec::with_capacity(tiles.len());
    for tile in tiles {
        let (width, height) = tile.size.cells();
        let mut row = 0;
        let (column, row) = 'search: loop {
            if rows.len() < row + height {
                rows.resize(row + height, 0);
            }
            for column in 0..=columns - width {
                let mask = (((1u16 << width) - 1) << column) as u8;
                if rows[row..row + height]
                    .iter()
                    .all(|occupied| occupied & mask == 0)
                {
                    for occupied in &mut rows[row..row + height] {
                        *occupied |= mask;
                    }
                    break 'search (column, row);
                }
            }
            row += 1;
        };
        placements.push(TilePlacement {
            id: tile.id,
            column,
            row,
            width,
            height,
        });
    }
    placements
}
