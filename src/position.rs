use std::collections::HashSet;

use icarus_board::{
    board::{Board, TerminalState},
    r#move::Move,
};
use icarus_common::piece::Piece;

use crate::{
    nnue::network::Nnue,
    score::Score,
    search::params::{mat_scale, mat_scaling_base},
};

#[derive(Clone)]
pub struct Position {
    board: Board,
    /// Previously played boards. `history[0]` is the starting position.
    history: Vec<Board>,
    /// Zobrist hashes of the previous positions, used for repetition detection.
    /// We store these separately to the history, because pre-root hashes are pruned,
    /// by only keeping the ones that occurred twice already. This way, we don't immediately
    /// return draw scores on positions that occurred before root.
    hashes: Vec<u64>,
    moves: Vec<Option<(Piece, Move)>>,
}

impl Position {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            history: vec![],
            hashes: vec![],
            moves: vec![],
        }
    }

    pub fn make_move(&mut self, mv: Move, nnue: Option<&mut Nnue>) {
        let piece = mv.piece_type(&self.board);
        if let Some(nnue) = nnue {
            nnue.make_move(&self.board, mv);
        }
        self.history.push(self.board);
        self.hashes.push(self.board.hash());
        self.board.make_move(mv);
        self.moves.push(Some((piece, mv)));
    }

    pub fn make_null_move(&mut self) {
        self.history.push(self.board);
        self.hashes.push(self.board.hash());
        self.board.make_null_move();
        self.moves.push(None);
    }

    pub fn unmake_move(&mut self, nnue: Option<&mut Nnue>) {
        if let Some(nnue) = nnue {
            nnue.unmake_move();
        }
        self.board = self.history.pop().unwrap();
        self.moves.pop();
        self.hashes.pop();
    }

    pub fn unmake_null_move(&mut self) {
        self.board = self.history.pop().unwrap();
        self.moves.pop();
        self.hashes.pop();
    }

    pub fn prune_preroot_hashes(&mut self) {
        let mut seen = HashSet::new();
        self.hashes.retain(|&h| !seen.insert(h));
    }

    pub fn eval(&self, nnue: &mut Nnue, mat_scaling: bool) -> Score {
        nnue.update(&self.board);
        let eval = nnue.eval(self.board.stm());

        let scale = if mat_scaling {
            mat_scaling_base()
                + Piece::all()
                    .map(|pt| self.board.pieces(pt).popcnt() as i32 * mat_scale(pt))
                    .sum::<i32>()
        } else {
            32768
        };

        Score::clamp_nomate((eval * scale / 32768).clamp(i16::MIN as i32, i16::MAX as i32) as i16)
    }

    pub fn prev_move(&self, ply: usize) -> Option<(Piece, Move)> {
        self.moves
            .len()
            .checked_sub(ply)
            .and_then(|i| self.moves[i])
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn repetition(&self) -> bool {
        self.hashes
            .iter()
            .rev()
            .take(self.board.halfmove_clock() as usize)
            .any(|&b| b == self.board.hash())
    }

    pub fn is_draw(&self) -> bool {
        self.board.terminal_state() == Some(TerminalState::Draw) || self.repetition()
    }
}
