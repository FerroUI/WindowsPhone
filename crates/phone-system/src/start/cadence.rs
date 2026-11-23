//! Pure deterministic live-tile scheduling.
pub(crate) fn cycle(now: i64, period: u64, seed: u64, count: usize, enabled: bool) -> usize {
    if !enabled || count <= 1 {
        return 0;
    }
    let tick = now.unsigned_abs().saturating_add(seed % period.max(1)) / period.max(1);
    (tick % count as u64) as usize
}

/// Exactly one People cell advances at each tick. Other cells retain both
/// their content identity and their mounted native controls.
pub(crate) fn person(now: i64, cell: usize, count: usize, enabled: bool) -> usize {
    if count == 0 {
        return 0;
    }
    let tick = if enabled { now.unsigned_abs() / 4 } else { 0 };
    let advances = tick.saturating_add(5 - cell.min(5) as u64) / 6;
    ((cell as u64 + advances) % count as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::{cycle, person};

    #[test]
    fn cycles_are_bounded_and_disabled_tiles_stay_still() {
        for now in [i64::MIN, -1, 0, 1, 17, 400, i64::MAX] {
            assert!(cycle(now, 17, 1, 3, true) < 3);
            assert_eq!(cycle(now, 17, 1, 3, false), 0);
            assert_eq!(cycle(now, 0, 1, 0, true), 0);
        }
    }

    #[test]
    fn people_updates_only_one_cell_per_tick() {
        for tick in 1..100 {
            let changed = (0..6)
                .filter(|cell| {
                    person(tick * 4, *cell, 7, true) != person((tick - 1) * 4, *cell, 7, true)
                })
                .count();
            assert_eq!(changed, 1);
        }
        for cell in 0..6 {
            assert_eq!(person(900, cell, 5, false), cell % 5);
        }
    }
}
