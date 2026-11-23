use super::{Game, Status, MAX_TIMESTAMP};

impl Game {
    pub fn validate(&self) -> Result<(), String> {
        let (width, height, mines) = self.difficulty.dimensions();
        let invalid =
            || "The saved minefield is inconsistent. Start a new game to reset it.".to_string();
        if self.seed == 0
            || self.cells.len() != width * height
            || self.cells.iter().filter(|cell| cell.flagged).count() > mines
            || self.cells.iter().any(|cell| cell.revealed && cell.flagged)
            || self
                .started_at
                .into_iter()
                .chain(self.finished_at)
                .any(|time| !(-MAX_TIMESTAMP..=MAX_TIMESTAMP).contains(&time))
        {
            return Err(invalid());
        }
        let Some(first) = self.first else {
            if self.status != Status::Ready
                || self.moves != 0
                || self.started_at.is_some()
                || self.finished_at.is_some()
                || self
                    .cells
                    .iter()
                    .any(|cell| cell.mine || cell.adjacent != 0 || cell.revealed)
            {
                return Err(invalid());
            }
            return Ok(());
        };
        if first >= self.cells.len()
            || !self.cells[first].revealed
            || self.started_at.is_none()
            || self.moves == 0
            || self.moves as usize > self.cells.iter().filter(|cell| cell.revealed).count()
        {
            return Err(invalid());
        }
        let expected = self.expected_cells(first);
        if self.cells.iter().zip(expected).any(|(cell, expected)| {
            cell.mine != expected.mine || cell.adjacent != expected.adjacent
        }) {
            return Err(invalid());
        }
        let hit = self.cells.iter().any(|cell| cell.mine && cell.revealed);
        let cleared = self.cells.iter().all(|cell| cell.mine || cell.revealed);
        let valid_status = match self.status {
            Status::Ready => false,
            Status::Playing => !hit && !cleared && self.finished_at.is_none(),
            Status::Won => !hit && cleared && self.finished_at.is_some(),
            Status::Lost => hit && self.finished_at.is_some(),
        };
        if !valid_status
            || self
                .finished_at
                .zip(self.started_at)
                .is_some_and(|(end, start)| end < start)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
