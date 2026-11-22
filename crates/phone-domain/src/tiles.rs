use crate::{AppId, Id};
use serde::{Deserialize, Serialize};

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
}
impl Tile {
    pub fn new(id: Id, app: AppId, size: TileSize) -> Self {
        Self {
            id,
            app,
            size,
            label: app.label().into(),
            children: Vec::new(),
        }
    }
    pub fn is_folder(&self) -> bool {
        !self.children.is_empty()
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
