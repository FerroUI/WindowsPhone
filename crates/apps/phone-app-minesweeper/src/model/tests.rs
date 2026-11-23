use super::*;

#[test]
fn every_first_cell_is_safe_and_mine_counts_are_exact() {
    for difficulty in Difficulty::ALL {
        let (width, height, mines) = difficulty.dimensions();
        for first in 0..width * height {
            let mut game = Game::new(difficulty, 123_456);
            assert!(game.reveal(first, 100));
            assert_eq!(game.cells[first].adjacent, 0);
            assert!(!game.cells[first].mine);
            assert_eq!(game.cells.iter().filter(|cell| cell.mine).count(), mines);
            assert_ne!(game.status, Status::Lost);
            game.validate().unwrap();
        }
    }
}

#[test]
fn deterministic_replay_and_resumed_model_are_identical() {
    let mut first = Game::new(Difficulty::Expert, u32::MAX);
    let mut second = first.clone();
    first.reveal(71, 20);
    second.reveal(71, 20);
    assert_eq!(first, second);
    let hidden = first.cells.iter().position(|cell| !cell.revealed).unwrap();
    first.flag(hidden);
    second.flag(hidden);
    assert_eq!(first, second);
    first.validate().unwrap();
}

#[test]
fn flood_fill_winning_score_is_recorded_once() {
    let mut save = Saved::default();
    save.reveal(0, 100).unwrap();
    for index in 0..save.game.cells.len() {
        if !save.game.cells[index].mine {
            save.reveal(index, 112).unwrap();
        }
    }
    assert_eq!(save.game.status, Status::Won);
    assert_eq!(save.game.remaining(), 0);
    assert_eq!(save.stats[0].won, 1);
    assert_eq!(save.stats[0].best_seconds, Some(12));
    assert!(!save.reveal(0, 140).unwrap());
    assert!(!save.flag(1).unwrap());
    assert_eq!(save.stats[0].won, 1);
    save.validate().unwrap();
}

#[test]
fn chord_reveals_only_when_flag_count_matches_and_wrong_flags_can_lose() {
    let mut good = Game::new(Difficulty::Beginner, 9182);
    good.reveal(0, 100);
    let numbered = (0..good.cells.len())
        .find(|index| {
            good.cells[*index].revealed
                && good.cells[*index].adjacent > 0
                && good
                    .neighbors(*index)
                    .any(|other| !good.cells[other].mine && !good.cells[other].revealed)
        })
        .unwrap();
    assert!(!good.reveal(numbered, 101));
    let mut bad = good.clone();
    for neighbor in good.neighbors(numbered).collect::<Vec<_>>() {
        if good.cells[neighbor].mine {
            good.flag(neighbor);
        }
    }
    assert!(good.reveal(numbered, 102));
    assert_ne!(good.status, Status::Lost);
    good.validate().unwrap();

    let count = usize::from(bad.cells[numbered].adjacent);
    let mut candidates: Vec<_> = bad
        .neighbors(numbered)
        .filter(|other| !bad.cells[*other].revealed)
        .collect();
    candidates.sort_by_key(|index| bad.cells[*index].mine);
    assert!(!bad.cells[candidates[0]].mine);
    for index in candidates.into_iter().take(count) {
        bad.flag(index);
    }
    assert!(bad.reveal(numbered, 102));
    assert_eq!(bad.status, Status::Lost);
    bad.validate().unwrap();
}

#[test]
fn flags_and_indices_are_bounded_and_corrupt_resume_is_rejected() {
    let mut game = Game::default();
    assert!(!game.flag(usize::MAX));
    assert!(!game.reveal(usize::MAX, 0));
    for index in 0..10 {
        assert!(game.flag(index));
    }
    assert!(!game.flag(11));
    assert!(!game.reveal(0, 0));
    game.flag(0);
    assert!(game.reveal(0, 100));
    let mut invalid = game.clone();
    invalid.cells[0].mine = true;
    assert!(invalid.validate().is_err());
    invalid = game.clone();
    invalid.cells.pop();
    assert!(invalid.validate().is_err());
    assert_eq!(game.elapsed(50), 0);
}

#[test]
fn imported_moves_and_times_are_bounded_and_elapsed_scores_are_json_exact() {
    let mut save = Saved::default();
    let original = save.clone();
    assert!(save.reveal(0, i64::MAX).is_err());
    assert_eq!(save, original);
    save.reveal(0, -MAX_TIMESTAMP).unwrap();
    let mut invalid = save.game.clone();
    invalid.moves = invalid.cells.iter().filter(|cell| cell.revealed).count() as u32 + 1;
    assert!(invalid.validate().is_err());
    invalid = save.game.clone();
    invalid.started_at = Some(i64::MIN);
    assert!(invalid.validate().is_err());
    for index in 0..save.game.cells.len() {
        if !save.game.cells[index].mine {
            save.reveal(index, MAX_TIMESTAMP).unwrap();
        }
    }
    assert_eq!(save.game.status, Status::Won);
    assert_eq!(save.stats[0].best_seconds, Some(u32::MAX));
    save.validate().unwrap();
}
