//! One transient clock per mounted shell. Scheduled changes never write storage.
use crate::session::Session;
use ferroui_view::prelude::*;
use phone_domain::{Route, Tile, TileTemplate, TILE_CARD_SECONDS};
use phone_state::PhoneViewModel;
use std::cell::Cell;

struct Clock {
    revision: Signal<i64>,
    deadline: Cell<Option<i64>>,
}
impl Default for Clock {
    fn default() -> Self {
        Self {
            revision: signal(0),
            deadline: Cell::new(None),
        }
    }
}

pub(super) fn configure(vm: &PhoneViewModel, ui: &Session, tiles: &[Tile]) {
    let clock = ui.state::<Clock>();
    // The signal only invalidates the view. Wall time comes from the host, so a
    // restore, clock correction or resume cannot retain a stale notification.
    clock.revision.get();
    let enabled = vm.data().settings.live_tiles;
    let rotate = vm.live_tiles_enabled() && ui.edit_tile.get().is_none();
    clock
        .deadline
        .set(next_tick(tiles, vm.now(), enabled, rotate));
}

pub(crate) fn tick(ui: &Session, now: i64) {
    let clock = ui.state::<Clock>();
    if matches!(*ui.last_route.borrow(), Route::Start | Route::Folder(_))
        && !ui.locked.get_untracked()
        && !ui.notification_open.get_untracked()
        && !ui.switcher_open.get_untracked()
        && clock.deadline.get().is_some_and(|deadline| now >= deadline)
    {
        // A rerender calculates the next actual boundary. No interval or task
        // remains owned by an individual tile after unpinning or app teardown.
        clock.deadline.set(None);
        clock.revision.set(now);
    }
}

fn boundary(now: i64, start: i64, period: u64) -> i64 {
    let elapsed = now.saturating_sub(start).max(0) as u64;
    now.saturating_add((period - elapsed % period) as i64)
}

fn next_tick(tiles: &[Tile], now: i64, enabled: bool, rotate: bool) -> Option<i64> {
    if !enabled {
        return None;
    }
    let mut next = None;
    let mut consider = |time: i64| {
        if time > now {
            next = Some(next.map_or(time, |old: i64| old.min(time)));
        }
    };
    for tile in tiles
        .iter()
        .flat_map(|tile| std::iter::once(tile).chain(tile.folder_tiles.iter()))
    {
        if !tile.appearance.live || tile.is_folder() {
            continue;
        }
        for card in &tile.notifications {
            consider(card.starts_at);
            if let Some(expiry) = card.expires_at {
                consider(expiry);
            }
        }
        if !rotate {
            continue;
        }
        let active = tile
            .notifications
            .iter()
            .rev()
            .filter(|card| card.active_at(now))
            .collect::<Vec<_>>();
        if tile.appearance.queue && active.len() > 1 {
            consider(boundary(now, active[0].starts_at, TILE_CARD_SECONDS));
        }
        if let Some(card) = tile.active_notification(now, true) {
            match &card.template {
                TileTemplate::Flip { .. } => consider(boundary(now, card.starts_at, 4)),
                TileTemplate::Cycle { photos, .. } if photos.len() > 1 => {
                    consider(boundary(now, card.starts_at, TILE_CARD_SECONDS))
                }
                _ => {}
            }
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_domain::{AppId, TileNotification, TileSize};
    fn tile() -> Tile {
        Tile::new(8, AppId::Weather, TileSize::Wide)
    }
    fn card(start: i64, end: Option<i64>) -> TileNotification {
        let mut value = TileNotification::new(
            "forecast",
            TileTemplate::Text {
                title: "rain".into(),
                body: "umbrella".into(),
            },
            start,
        );
        value.expires_at = end;
        value
    }
    #[test]
    fn deadlines_honor_schedule_and_expiry_without_motion() {
        let mut tile = tile();
        tile.notifications.push(card(110, Some(130)));
        assert_eq!(next_tick(&[tile.clone()], 100, true, false), Some(110));
        assert_eq!(next_tick(&[tile.clone()], 110, true, false), Some(130));
        assert_eq!(next_tick(&[tile.clone()], 130, true, false), None);
        assert_eq!(next_tick(&[tile.clone()], 100, false, true), None);
        tile.appearance.live = false;
        assert_eq!(next_tick(&[tile], 100, true, true), None);
    }
    #[test]
    fn static_cards_do_not_tick_and_queues_stop_when_only_one_remains() {
        let mut tile = tile();
        tile.notifications.push(card(100, None));
        assert_eq!(next_tick(&[tile.clone()], 101, true, true), None);
        let mut second = card(101, Some(116));
        second.tag = "second".into();
        tile.notifications.push(second);
        assert_eq!(next_tick(&[tile.clone()], 102, true, true), Some(109));
        assert_eq!(next_tick(&[tile.clone()], 109, true, true), Some(116));
        assert_eq!(next_tick(&[tile.clone()], 116, true, true), None);
        assert_eq!(next_tick(&[tile], 102, true, false), Some(116));
    }
}
