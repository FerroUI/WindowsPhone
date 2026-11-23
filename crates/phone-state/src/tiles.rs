use phone_domain::*;

pub(crate) fn mutate_tiles(data: &mut PhoneData, action: Action) -> Result<bool, String> {
    match action {
        Action::Pin(app) => {
            if data.is_pinned(app) {
                return Ok(false);
            }
            ensure_capacity(data)?;
            let id = data.allocate_id();
            data.tiles.push(Tile::new(id, app, TileSize::Medium));
            Ok(true)
        }
        Action::PinApp { app, path, title } => {
            validate_app_link(&app, &path, &title)?;
            if data.disabled_extensions.contains(&app) {
                return Err("Enable this app before pinning it.".into());
            }
            if data
                .app_links
                .iter()
                .any(|link| link.app == app && link.path == path)
            {
                return Ok(false);
            }
            ensure_capacity(data)?;
            if data.app_links.len() >= 512 || data.next_id > Id::MAX - 3 {
                return Err("The application link limit has been reached.".into());
            }
            let link_id = data.allocate_id();
            let tile_id = data.allocate_id();
            data.app_links.push(AppLink {
                id: link_id,
                app,
                path,
                title: title.trim().into(),
            });
            let title = data.app_links.last().unwrap().title.clone();
            data.tiles.push(Tile::secondary(
                tile_id,
                TileTarget::AppLink(link_id),
                title,
            ));
            Ok(true)
        }
        Action::PinTarget(target) => {
            if data.target_is_pinned(target) {
                return Ok(false);
            }
            let Some(label) = data.tile_target_label(target) else {
                return Err("That item no longer exists.".into());
            };
            ensure_capacity(data)?;
            let id = data.allocate_id();
            data.tiles.push(Tile::secondary(id, target, label));
            Ok(true)
        }
        Action::Unpin(id) => {
            let changed = take(data, id).is_some();
            data.tidy_folders();
            Ok(changed)
        }
        Action::ResizeTile(id) => Ok(find_mut(data, id).is_some_and(|tile| {
            tile.size = tile.size.next();
            true
        })),
        Action::SetTileSize(id, size) => Ok(find_mut(data, id).is_some_and(|tile| {
            let changed = tile.size != size;
            tile.size = size;
            changed
        })),
        Action::MoveTile { id, before } => Ok(move_tile(data, id, before, None)),
        Action::MoveTileRelative { id, target, after } => {
            if target == id {
                return Ok(false);
            }
            let Some(siblings) = data.tile_siblings(target) else {
                return Ok(false);
            };
            let Some(index) = siblings.iter().position(|tile| tile.id == target) else {
                return Ok(false);
            };
            let before = if after {
                siblings
                    .iter()
                    .skip(index + 1)
                    .find(|tile| tile.id != id)
                    .map(|tile| tile.id)
            } else {
                Some(target)
            };
            let destination = data.parent_folder(target);
            Ok(move_tile(data, id, before, Some(destination)))
        }
        Action::MoveFolderTile {
            folder,
            tile,
            before,
        } => {
            if data.parent_folder(tile) != Some(folder) {
                return Ok(false);
            }
            Ok(move_tile(data, tile, before, Some(Some(folder))))
        }
        Action::CreateFolder {
            source,
            target,
            name,
        } => Ok(create_folder(data, source, target, name)),
        Action::RenameFolder { id, name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err("Give your folder a name.".into());
            }
            let name: String = name.chars().take(64).collect();
            Ok(data
                .tiles
                .iter_mut()
                .find(|tile| tile.id == id && tile.is_folder())
                .is_some_and(|tile| {
                    let changed = tile.label != name;
                    tile.label = name;
                    changed
                }))
        }
        Action::RemoveFromFolder { id, app } => {
            let child = data
                .tile(id)
                .and_then(|folder| folder.folder_tiles.iter().find(|tile| tile.app == app))
                .map(|tile| tile.id);
            Ok(child.is_some_and(|child| extract(data, id, child)))
        }
        Action::ExtractFolderTile { folder, tile } => Ok(extract(data, folder, tile)),
        _ => Ok(false),
    }
}

fn ensure_capacity(data: &PhoneData) -> Result<(), String> {
    if data.tiles.len() >= 256 || data.all_tiles().count() >= 512 {
        Err("The Start screen has reached its tile limit.".into())
    } else {
        Ok(())
    }
}
fn find_mut(data: &mut PhoneData, id: Id) -> Option<&mut Tile> {
    for tile in &mut data.tiles {
        if tile.id == id {
            return Some(tile);
        }
        if let Some(child) = tile.folder_tiles.iter_mut().find(|child| child.id == id) {
            return Some(child);
        }
    }
    None
}
/// Keep the folder's legacy list until the operation completes, so an empty
/// folder is still recognizable and can be removed by tidy_folders.
fn take(data: &mut PhoneData, id: Id) -> Option<Tile> {
    if let Some(index) = data.tiles.iter().position(|tile| tile.id == id) {
        return Some(data.tiles.remove(index));
    }
    for folder in &mut data.tiles {
        if let Some(index) = folder.folder_tiles.iter().position(|tile| tile.id == id) {
            return Some(folder.folder_tiles.remove(index));
        }
    }
    None
}
fn leaves(tile: Tile) -> Vec<Tile> {
    if tile.is_folder() {
        tile.folder_tiles
    } else {
        vec![tile]
    }
}

fn move_tile(data: &mut PhoneData, id: Id, before: Option<Id>, folder: Option<Option<Id>>) -> bool {
    if before == Some(id)
        || data.tile(id).is_none()
        || before.is_some_and(|target| data.tile(target).is_none())
    {
        return false;
    }
    let destination = folder.unwrap_or_else(|| {
        before
            .and_then(|target| data.parent_folder(target))
            .or_else(|| {
                if before.is_none() {
                    data.parent_folder(id)
                } else {
                    None
                }
            })
    });
    if destination == Some(id) {
        return false;
    }
    if let (Some(folder), Some(target)) = (folder, before) {
        if data.parent_folder(target) != folder {
            return false;
        }
    }
    let snapshot = data.tiles.clone();
    let tile = take(data, id).unwrap();
    if let Some(folder) = destination {
        let Some(destination) = data.tiles.iter_mut().find(|tile| tile.id == folder) else {
            data.tiles = snapshot;
            return false;
        };
        let at = before
            .and_then(|target| {
                destination
                    .folder_tiles
                    .iter()
                    .position(|tile| tile.id == target)
            })
            .unwrap_or(destination.folder_tiles.len());
        let children = leaves(tile);
        if destination.folder_tiles.len() + children.len() > 256 {
            data.tiles = snapshot;
            return false;
        }
        destination.folder_tiles.splice(at..at, children);
        destination.sync_children();
    } else {
        let at = before
            .and_then(|target| data.tiles.iter().position(|tile| tile.id == target))
            .unwrap_or(data.tiles.len());
        data.tiles.insert(at, tile);
    }
    data.tidy_folders();
    if data.tiles.len() > 256 {
        data.tiles = snapshot;
        return false;
    }
    data.tiles != snapshot
}

fn create_folder(data: &mut PhoneData, source: Id, target: Id, name: String) -> bool {
    if source == target
        || data.tile(source).is_none()
        || data.tile(target).is_none()
        || data.parent_folder(target) == Some(source)
    {
        return false;
    }
    if data.parent_folder(target).is_some() {
        return move_tile(data, source, Some(target), None);
    }
    if data.parent_folder(source) == Some(target) {
        return false;
    }
    let snapshot = data.tiles.clone();
    let next_id = data.next_id;
    let source_tile = take(data, source).unwrap();
    let source_tiles = leaves(source_tile);
    let Some(at) = data.tiles.iter().position(|tile| tile.id == target) else {
        data.tiles = snapshot;
        return false;
    };
    if data.tiles[at].is_folder() {
        if data.tiles[at].folder_tiles.len() + source_tiles.len() > 256 {
            data.tiles = snapshot;
            return false;
        }
        data.tiles[at].folder_tiles.extend(source_tiles);
        data.tiles[at].sync_children();
    } else {
        let original = data.tiles.remove(at);
        let size = if original.size == TileSize::Small {
            TileSize::Medium
        } else {
            original.size
        };
        let mut folder = Tile::new(data.allocate_id(), original.app, size);
        folder.label = if name.trim().is_empty() {
            "Folder".into()
        } else {
            name.trim().chars().take(64).collect()
        };
        folder.folder_tiles.push(original);
        folder.folder_tiles.extend(source_tiles);
        folder.sync_children();
        data.tiles.insert(at, folder);
    }
    data.tidy_folders();
    if data.all_tiles().count() > 512 {
        data.tiles = snapshot;
        data.next_id = next_id;
        return false;
    }
    true
}
fn extract(data: &mut PhoneData, folder: Id, id: Id) -> bool {
    if data.parent_folder(id) != Some(folder) {
        return false;
    }
    if data.tiles.len() >= 256
        && data
            .tile(folder)
            .is_some_and(|tile| tile.folder_tiles.len() > 2)
    {
        return false;
    }
    let child = take(data, id).unwrap();
    let at = data
        .tiles
        .iter()
        .position(|tile| tile.id == folder)
        .map(|at| at + 1)
        .unwrap_or(data.tiles.len());
    data.tiles.insert(at, child);
    data.tidy_folders();
    true
}
