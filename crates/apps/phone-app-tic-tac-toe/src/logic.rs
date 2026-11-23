//! A bounded, allocation-free minimax player. 0 = empty, 1 = human, 2 = phone.
const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];
const ORDER: [usize; 9] = [4, 0, 2, 6, 8, 1, 3, 5, 7];

/// 0 = in progress, 1/2 = winner, 3 = draw.
pub fn outcome(board: &[u8; 9]) -> u8 {
    for [a, b, c] in LINES {
        if board[a] != 0 && board[a] == board[b] && board[b] == board[c] {
            return board[a];
        }
    }
    if board.contains(&0) {
        0
    } else {
        3
    }
}

pub fn best_move(board: &[u8; 9]) -> Option<usize> {
    if outcome(board) != 0 {
        return None;
    }
    let mut best = None;
    let mut value = i32::MIN;
    for index in ORDER {
        if board[index] != 0 {
            continue;
        }
        let mut next = *board;
        next[index] = 2;
        let score = minimax(next, 1, 0);
        if score > value {
            value = score;
            best = Some(index);
        }
    }
    best
}

fn minimax(board: [u8; 9], turn: u8, depth: i32) -> i32 {
    match outcome(&board) {
        1 => return depth - 10,
        2 => return 10 - depth,
        3 => return 0,
        _ => {}
    }
    let mut value = if turn == 2 { i32::MIN } else { i32::MAX };
    for index in ORDER {
        if board[index] != 0 {
            continue;
        }
        let mut next = board;
        next[index] = turn;
        let score = minimax(next, 3 - turn, depth + 1);
        value = if turn == 2 {
            value.max(score)
        } else {
            value.min(score)
        };
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_never_loses_against_any_human_sequence() {
        fn explore(board: [u8; 9]) {
            if outcome(&board) != 0 {
                assert_ne!(outcome(&board), 1);
                return;
            }
            for index in 0..9 {
                if board[index] != 0 {
                    continue;
                }
                let mut next = board;
                next[index] = 1;
                assert_ne!(outcome(&next), 1, "human won on {next:?}");
                if let Some(reply) = best_move(&next) {
                    next[reply] = 2;
                }
                explore(next);
            }
        }
        explore([0; 9]);
    }
    #[test]
    fn completed_board_has_no_next_move() {
        assert_eq!(best_move(&[2, 2, 2, 1, 1, 0, 0, 0, 0]), None);
    }
}
