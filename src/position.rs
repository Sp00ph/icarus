use icarus_board::board::{Board, TerminalState};
use icarus_common::r#move::Move;

#[derive(Clone)]
pub struct Position {
    board: Board,
    /// Previously played boards. `history[0]` is the starting position.
    history: Vec<Board>,
}

impl Position {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            history: vec![],
        }
    }

    pub fn make_move(&mut self, mv: Move) {
        self.history.push(self.board);
        self.board.make_move(mv);
    }

    pub fn unmake_move(&mut self) {
        self.board = self.history.pop().unwrap();
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn repetition(&self) -> bool {
        // It's important for codegen quality here that we skip(3).take(max(hm - 3, 0)) instead of take(hm).skip(3)
        self.history
            .iter()
            .rev()
            .skip(3)
            .take((self.board.halfmove_clock() as usize).saturating_sub(3))
            .step_by(2)
            .any(|b| b.hash() == self.board.hash())
    }

    pub fn is_draw(&self) -> bool {
        self.board.terminal_state() == Some(TerminalState::Draw) || self.repetition()
    }
}
