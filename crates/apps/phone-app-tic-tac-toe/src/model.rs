use crate::logic;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Phone,
    TwoPlayers,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Phone => "against phone",
            Self::TwoPlayers => "two players",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub moves: Vec<u8>,
}

impl Game {
    pub fn board(&self) -> Result<[u8; 9], String> {
        if self.moves.len() > 9 {
            return Err("The saved game contains too many moves.".into());
        }
        let mut board = [0; 9];
        for (ply, &cell) in self.moves.iter().enumerate() {
            let index = usize::from(cell);
            if index >= 9 || board[index] != 0 || logic::outcome(&board) != 0 {
                return Err("The saved game contains an invalid move.".into());
            }
            if self.mode == Mode::Phone && ply % 2 == 1 && logic::best_move(&board) != Some(index) {
                return Err("The saved phone reply is invalid.".into());
            }
            board[index] = 1 + (ply % 2) as u8;
        }
        if self.mode == Mode::Phone && self.moves.len() % 2 == 1 && logic::outcome(&board) == 0 {
            return Err("The saved game has an incomplete phone turn.".into());
        }
        Ok(board)
    }

    pub fn play(&mut self, index: usize) -> Result<bool, String> {
        let mut board = self.board()?;
        if index >= 9 || board[index] != 0 || logic::outcome(&board) != 0 {
            return Ok(false);
        }
        board[index] = 1 + (self.moves.len() % 2) as u8;
        self.moves.push(index as u8);
        if self.mode == Mode::Phone {
            if let Some(reply) = logic::best_move(&board) {
                self.moves.push(reply as u8);
            }
        }
        Ok(true)
    }

    pub fn can_undo(&self) -> bool {
        !self.moves.is_empty() && self.board().is_ok_and(|board| logic::outcome(&board) == 0)
    }

    pub fn undo(&mut self) -> bool {
        if !self.can_undo() {
            return false;
        }
        let count = if self.mode == Mode::Phone { 2 } else { 1 };
        self.moves.truncate(self.moves.len().saturating_sub(count));
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u32,
    pub game: Game,
    pub phone_scores: [u32; 3],
    pub local_scores: [u32; 3],
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            game: Game::default(),
            phone_scores: [0; 3],
            local_scores: [0; 3],
        }
    }
}

impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("This saved game uses an unsupported version.".into());
        }
        self.game.board().map(|_| ())
    }

    pub fn play(&mut self, index: usize) -> Result<bool, String> {
        self.validate()?;
        if !self.game.play(index)? {
            return Ok(false);
        }
        let outcome = logic::outcome(&self.game.board()?);
        if outcome != 0 {
            let scores = if self.game.mode == Mode::Phone {
                &mut self.phone_scores
            } else {
                &mut self.local_scores
            };
            scores[usize::from(outcome - 1)] = scores[usize::from(outcome - 1)].saturating_add(1);
        }
        Ok(true)
    }

    pub fn restart(&mut self, mode: Mode) {
        self.game = Game {
            mode,
            moves: vec![],
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_players_complete_once_and_cannot_undo_a_recorded_result() {
        let mut save = Saved::default();
        save.restart(Mode::TwoPlayers);
        for index in [0, 3, 1, 4, 2] {
            assert!(save.play(index).unwrap());
        }
        assert_eq!(save.local_scores, [1, 0, 0]);
        assert!(!save.play(8).unwrap());
        assert!(!save.game.undo());
        assert_eq!(save.local_scores, [1, 0, 0]);
    }

    #[test]
    fn phone_undo_removes_the_complete_turn_without_changing_scores() {
        let mut save = Saved::default();
        save.play(0).unwrap();
        assert_eq!(save.game.moves.len(), 2);
        assert!(save.game.undo());
        assert_eq!(save.game.board().unwrap(), [0; 9]);
        assert_eq!(save.phone_scores, [0; 3]);
    }

    #[test]
    fn restore_rejects_overwrites_and_moves_after_a_win() {
        let mut save = Saved::default();
        save.game.mode = Mode::TwoPlayers;
        save.game.moves = vec![0, 0];
        assert!(save.validate().is_err());
        save.game.moves = vec![0, 3, 1, 4, 2, 8];
        assert!(save.validate().is_err());
        save.game.mode = Mode::Phone;
        save.game.moves = vec![0];
        assert!(save.validate().is_err());
    }
}
