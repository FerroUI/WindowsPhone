use phone_domain::*;

pub(crate) fn mutate_tiles(data: &mut PhoneData, action: Action) -> Result<bool, String> {
    match action {
        Action::Pin(app) => {
            if data.is_pinned(app) {
                return Ok(false);
            }
            if data.tiles.len() >= 256 {
                return Err("The Start screen has reached its tile limit.".into());
            }
            let id = data.allocate_id();
            data.tiles.push(Tile::new(id, app, TileSize::Medium));
            Ok(true)
        }
        Action::Unpin(id) => {
            let count = data.tiles.len();
            data.tiles.retain(|t| t.id != id);
            Ok(count != data.tiles.len())
        }
        Action::ResizeTile(id) => {
            if let Some(tile) = data.tiles.iter_mut().find(|t| t.id == id) {
                tile.size = tile.size.next();
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Action::SetTileSize(id, size) => {
            if let Some(tile) = data.tiles.iter_mut().find(|t| t.id == id && t.size != size) {
                tile.size = size;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Action::MoveTile { id, before } => {
            if before == Some(id) {
                return Ok(false);
            }
            let Some(index) = data.tiles.iter().position(|t| t.id == id) else {
                return Ok(false);
            };
            if before.is_some_and(|target| !data.tiles.iter().any(|t| t.id == target)) {
                return Ok(false);
            }
            let old_order = data.tiles.iter().map(|t| t.id).collect::<Vec<_>>();
            let tile = data.tiles.remove(index);
            let target = before
                .and_then(|target| data.tiles.iter().position(|t| t.id == target))
                .unwrap_or(data.tiles.len());
            data.tiles.insert(target, tile);
            Ok(data.tiles.iter().map(|t| t.id).ne(old_order))
        }
        Action::CreateFolder {
            source,
            target,
            name,
        } => {
            if source == target {
                return Ok(false);
            }
            let Some(source_tile) = data.tile(source).cloned() else {
                return Ok(false);
            };
            let Some(target_index) = data.tiles.iter().position(|t| t.id == target) else {
                return Ok(false);
            };
            let tile = &mut data.tiles[target_index];
            if tile.children.is_empty() {
                tile.children.push(tile.app);
            }
            let children = if source_tile.children.is_empty() {
                vec![source_tile.app]
            } else {
                source_tile.children
            };
            for app in children {
                if !tile.children.contains(&app) {
                    tile.children.push(app);
                }
            }
            tile.label = if name.trim().is_empty() {
                "Folder".into()
            } else {
                name.trim().chars().take(64).collect()
            };
            if tile.size == TileSize::Small {
                tile.size = TileSize::Medium;
            }
            data.tiles.retain(|t| t.id != source);
            Ok(true)
        }
        Action::RenameFolder { id, name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err("Give your folder a name.".into());
            }
            if let Some(tile) = data
                .tiles
                .iter_mut()
                .find(|t| t.id == id && t.is_folder() && t.label != name)
            {
                tile.label = name.chars().take(64).collect();
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Action::RemoveFromFolder { id, app } => {
            let Some(index) = data
                .tiles
                .iter()
                .position(|t| t.id == id && t.children.contains(&app))
            else {
                return Ok(false);
            };
            data.tiles[index].children.retain(|id| *id != app);
            if data.tiles[index].children.len() == 1 {
                let app = data.tiles[index].children[0];
                data.tiles[index].app = app;
                data.tiles[index].label = app.label().into();
                data.tiles[index].children.clear();
            }
            let new_id = data.allocate_id();
            data.tiles
                .insert(index + 1, Tile::new(new_id, app, TileSize::Medium));
            Ok(true)
        }
        _ => Ok(false),
    }
}
