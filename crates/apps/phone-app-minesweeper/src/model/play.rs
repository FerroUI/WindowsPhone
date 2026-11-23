use super::{Game, Status, MAX_TIMESTAMP};

impl Game {
    pub fn flag(&mut self, index: usize) -> bool {
        if self.complete() || index >= self.cells.len() || self.cells[index].revealed {
            return false;
        }
        if !self.cells[index].flagged && self.remaining() == 0 {
            return false;
        }
        self.cells[index].flagged = !self.cells[index].flagged;
        true
    }

    pub fn reveal(&mut self, index: usize, now: i64) -> bool {
        if self.complete()
            || index >= self.cells.len()
            || self.cells[index].flagged
            || !(-MAX_TIMESTAMP..=MAX_TIMESTAMP).contains(&now)
        {
            return false;
        }
        if self.first.is_none() {
            self.place_mines(index);
            self.first = Some(index);
            self.started_at = Some(now);
            self.status = Status::Playing;
        }
        let mut targets = vec![index];
        if self.cells[index].revealed {
            if self.cells[index].adjacent == 0 {
                return false;
            }
            let neighbors: Vec<_> = self.neighbors(index).collect();
            let flagged = neighbors
                .iter()
                .filter(|other| self.cells[**other].flagged)
                .count();
            if flagged != usize::from(self.cells[index].adjacent) {
                return false;
            }
            targets = neighbors
                .into_iter()
                .filter(|other| !self.cells[*other].flagged && !self.cells[*other].revealed)
                .collect();
            if targets.is_empty() {
                return false;
            }
        }
        let mut changed = false;
        while let Some(next) = targets.pop() {
            if self.cells[next].revealed || self.cells[next].flagged {
                continue;
            }
            self.cells[next].revealed = true;
            changed = true;
            if self.cells[next].mine {
                self.status = Status::Lost;
                self.finished_at = Some(now.max(self.started_at.unwrap_or(now)));
                break;
            }
            if self.cells[next].adjacent == 0 {
                targets.extend(
                    self.neighbors(next).filter(|other| {
                        !self.cells[*other].revealed && !self.cells[*other].flagged
                    }),
                );
            }
        }
        if changed {
            self.moves = self.moves.saturating_add(1);
            if self.status != Status::Lost
                && self.cells.iter().all(|cell| cell.mine || cell.revealed)
            {
                self.status = Status::Won;
                self.finished_at = Some(now.max(self.started_at.unwrap_or(now)));
                for cell in &mut self.cells {
                    if cell.mine {
                        cell.flagged = true;
                    }
                }
            }
        }
        changed
    }
}
