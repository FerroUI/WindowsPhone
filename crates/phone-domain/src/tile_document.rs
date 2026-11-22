use crate::{AppId, Id, PhoneData, Tile, TileSize, TileTarget};
use std::collections::HashSet;

impl PhoneData {
    pub fn all_tiles(&self) -> impl Iterator<Item = &Tile> {
        self.tiles
            .iter()
            .flat_map(|tile| std::iter::once(tile).chain(tile.folder_tiles.iter()))
    }
    pub fn parent_folder(&self, id: Id) -> Option<Id> {
        self.tiles
            .iter()
            .find(|tile| tile.folder_tiles.iter().any(|child| child.id == id))
            .map(|tile| tile.id)
    }
    pub fn tile_siblings(&self, id: Id) -> Option<&[Tile]> {
        if self.tiles.iter().any(|tile| tile.id == id) {
            return Some(&self.tiles);
        }
        self.tiles
            .iter()
            .find(|tile| tile.folder_tiles.iter().any(|child| child.id == id))
            .map(|folder| folder.folder_tiles.as_slice())
    }
    pub fn target_is_pinned(&self, target: TileTarget) -> bool {
        self.all_tiles().any(|tile| tile.target == Some(target))
    }
    pub fn tile_target_label(&self, target: TileTarget) -> Option<String> {
        match target {
            TileTarget::Contact(id) => self.contact(id).map(|contact| contact.name.clone()),
            TileTarget::Note(id) => self.note(id).map(|note| note.title.clone()),
            TileTarget::Bookmark(id) => self
                .bookmarks
                .iter()
                .find(|bookmark| bookmark.id == id)
                .map(|bookmark| bookmark.title.clone()),
        }
    }
    /// A folder is exactly one level deep. Legacy folders migrate into full child
    /// records once, assigning IDs above every existing document identifier.
    pub(crate) fn normalize_tiles(&mut self) -> Result<(), String> {
        if self.all_tiles().count() > 512 {
            return Err("The Start screen contains too many tiles.".into());
        }
        let mut ids = HashSet::new();
        let mut max = self.next_id.saturating_sub(1);
        for tile in &self.tiles {
            if !ids.insert(tile.id) {
                return Err("Duplicate Start tile identifier.".into());
            }
            max = max.max(tile.id);
            if tile.folder_tiles.len() > 256 {
                return Err("This folder contains too many tiles.".into());
            }
            for child in &tile.folder_tiles {
                if child.is_folder() {
                    return Err("Nested Start folders are not supported.".into());
                }
                if child.id == 0 || !ids.insert(child.id) {
                    return Err("Duplicate or zero folder tile identifier.".into());
                }
                max = max.max(child.id);
            }
        }
        if max == Id::MAX {
            return Err("The tile identifier range is exhausted.".into());
        }
        self.next_id = self.next_id.max(max + 1);
        let mut next = self.next_id;
        for tile in &mut self.tiles {
            if tile.folder_tiles.is_empty() && !tile.children.is_empty() {
                let mut apps = HashSet::new();
                for app in tile
                    .children
                    .clone()
                    .into_iter()
                    .filter(|app| apps.insert(*app))
                {
                    if next == Id::MAX {
                        return Err("The tile identifier range is exhausted.".into());
                    }
                    tile.folder_tiles
                        .push(Tile::new(next, app, TileSize::Small));
                    next += 1;
                }
            }
            if !tile.folder_tiles.is_empty() {
                tile.target = None;
                tile.sync_children();
            }
        }
        self.next_id = next;
        if self.all_tiles().count() > 512 {
            return Err("The Start screen contains too many tiles.".into());
        }
        for tile in self.all_tiles() {
            if let Some(target) = tile.target {
                if self.tile_target_label(target).is_none() {
                    return Err("A pinned tile refers to an item missing from the backup.".into());
                }
            }
        }
        Ok(())
    }
    /// Keep secondary labels current and discard shortcuts only when their
    /// underlying contact, note, bookmark, or installed app has been removed.
    pub fn clean_tile_targets(&mut self) {
        let valid = self
            .all_tiles()
            .filter_map(|tile| {
                tile.target.and_then(|target| {
                    self.tile_target_label(target)
                        .map(|label| (tile.id, (target.app(), label)))
                })
            })
            .collect::<std::collections::HashMap<_, _>>();
        let installed = self.installed.iter().copied().collect::<HashSet<AppId>>();
        fn keep(
            tile: &mut Tile,
            valid: &std::collections::HashMap<Id, (AppId, String)>,
            installed: &HashSet<AppId>,
        ) -> bool {
            if tile.target.is_some() {
                if let Some((app, label)) = valid.get(&tile.id) {
                    tile.app = *app;
                    tile.label = label.clone();
                } else {
                    return false;
                }
            }
            installed.contains(&tile.app)
        }
        self.tiles.retain_mut(|tile| {
            if tile.is_folder() {
                tile.folder_tiles
                    .retain_mut(|child| keep(child, &valid, &installed));
                if tile.folder_tiles.is_empty() {
                    false
                } else {
                    tile.sync_children();
                    true
                }
            } else {
                keep(tile, &valid, &installed)
            }
        });
        self.tidy_folders();
    }
    pub fn tidy_folders(&mut self) {
        let old = std::mem::take(&mut self.tiles);
        self.tiles = old
            .into_iter()
            .filter_map(|mut tile| {
                if tile.is_folder() {
                    match tile.folder_tiles.len() {
                        0 => None,
                        1 => tile.folder_tiles.pop(),
                        _ => {
                            tile.sync_children();
                            Some(tile)
                        }
                    }
                } else {
                    Some(tile)
                }
            })
            .collect();
    }
}
