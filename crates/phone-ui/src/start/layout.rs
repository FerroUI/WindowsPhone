use phone_domain::{Id, TilePlacement};

pub(super) fn rows(placements: &[TilePlacement]) -> usize {
    placements
        .iter()
        .map(|tile| tile.row + tile.height)
        .max()
        .unwrap_or(0)
}

/// Insert the open folder at a complete row boundary. A medium tile spanning
/// that boundary advances it, keeping every original rectangle collision-free.
pub(super) fn folder_band(placements: &[TilePlacement], id: Id) -> Option<usize> {
    let tile = placements.iter().find(|tile| tile.id == id)?;
    let mut after = tile.row + tile.height;
    loop {
        let next = placements
            .iter()
            .filter(|tile| tile.row < after && tile.row + tile.height > after)
            .map(|tile| tile.row + tile.height)
            .max();
        match next {
            Some(next) => after = next,
            None => return Some(after),
        }
    }
}
