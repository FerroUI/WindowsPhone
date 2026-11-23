//! Bounded, deterministic Minesweeper state. No host or UI dependency.
mod board;
mod play;
#[cfg(test)]
mod tests;
mod validation;

use serde::{Deserialize, Serialize};

const MAX_TIMESTAMP: i64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Beginner,
    Intermediate,
    Expert,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Beginner, Self::Intermediate, Self::Expert];

    pub fn dimensions(self) -> (usize, usize, usize) {
        match self {
            Self::Beginner => (8, 8, 10),
            Self::Intermediate => (10, 10, 18),
            Self::Expert => (12, 12, 30),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Expert => "expert",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Beginner => 0,
            Self::Intermediate => 1,
            Self::Expert => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub mine: bool,
    pub adjacent: u8,
    pub revealed: bool,
    pub flagged: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    #[default]
    Ready,
    Playing,
    Won,
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub difficulty: Difficulty,
    pub seed: u32,
    pub first: Option<usize>,
    pub cells: Vec<Cell>,
    pub status: Status,
    pub moves: u32,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new(Difficulty::Beginner, 1)
    }
}

impl Game {
    pub fn new(difficulty: Difficulty, seed: u32) -> Self {
        let (width, height, _) = difficulty.dimensions();
        Self {
            difficulty,
            seed: seed.max(1),
            first: None,
            cells: vec![Cell::default(); width * height],
            status: Status::Ready,
            moves: 0,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn remaining(&self) -> usize {
        self.difficulty
            .dimensions()
            .2
            .saturating_sub(self.cells.iter().filter(|cell| cell.flagged).count())
    }

    pub fn elapsed(&self, now: i64) -> u32 {
        self.started_at
            .map(|start| {
                self.finished_at
                    .unwrap_or(now)
                    .saturating_sub(start)
                    .clamp(0, i64::from(u32::MAX)) as u32
            })
            .unwrap_or(0)
    }

    pub fn complete(&self) -> bool {
        matches!(self.status, Status::Won | Status::Lost)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub won: u32,
    pub lost: u32,
    pub best_seconds: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub game: Game,
    pub stats: [Stats; 3],
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            game: Game::default(),
            stats: [Stats::default(); 3],
        }
    }
}

impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This saved game uses an unsupported version.".into());
        }
        self.game.validate()
    }

    pub fn reveal(&mut self, index: usize, now: i64) -> Result<bool, String> {
        self.validate()?;
        if !(-MAX_TIMESTAMP..=MAX_TIMESTAMP).contains(&now) {
            return Err("The phone clock is outside the supported timestamp range.".into());
        }
        let before = self.game.status;
        let changed = self.game.reveal(index, now);
        if changed && before != self.game.status && self.game.complete() {
            let stats = &mut self.stats[self.game.difficulty.index()];
            if self.game.status == Status::Won {
                stats.won = stats.won.saturating_add(1);
                let elapsed = self.game.elapsed(now);
                stats.best_seconds = Some(
                    stats
                        .best_seconds
                        .map_or(elapsed, |previous| previous.min(elapsed)),
                );
            } else {
                stats.lost = stats.lost.saturating_add(1);
            }
        }
        Ok(changed)
    }

    pub fn flag(&mut self, index: usize) -> Result<bool, String> {
        self.validate()?;
        Ok(self.game.flag(index))
    }

    pub fn restart(&mut self, difficulty: Difficulty, seed: u32) {
        self.game = Game::new(difficulty, seed);
    }
}
