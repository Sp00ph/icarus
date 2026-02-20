use icarus_board::{board::Board, r#move::Move};
use icarus_common::piece::Piece;

use crate::search::history::{MAX_HIST_VALUE, apply_gravity};

pub struct ContHist {
    /// [stm][prev piece][prev dst][piece][dst]
    data: [[[[[i16; 64]; 6]; 64]; 6]; 2],
}

impl ContHist {
    fn bonus(depth: i16) -> i32 {
        let bonus_base = 128;
        let bonus_scale = 128;
        let bonus_max = 2048;
        (bonus_base + (depth as i32) * bonus_scale).min(bonus_max)
    }

    fn malus(depth: i16) -> i32 {
        let malus_base = 128;
        let malus_scale = 128;
        let malus_max = 2048;
        (malus_base + (depth as i32) * malus_scale).min(malus_max)
    }

    pub fn get(&self, board: &Board, mv: Move, prev: Option<(Piece, Move)>) -> i16 {
        prev.map_or(0, |prev| {
            let (stm, prev_piece, prev_to) = (board.stm(), prev.0, prev.1.to());
            let (piece, to) = (board.piece_on(mv.from()).unwrap(), mv.to());
            self.data[stm][prev_piece][prev_to][piece][to]
        })
    }

    fn get_mut(
        &mut self,
        board: &Board,
        mv: Move,
        prev: Option<(Piece, Move)>,
    ) -> Option<&mut i16> {
        prev.map(|prev| {
            let (stm, prev_piece, prev_to) = (board.stm(), prev.0, prev.1.to());
            let (piece, to) = (board.piece_on(mv.from()).unwrap(), mv.to());
            &mut self.data[stm][prev_piece][prev_to][piece][to]
        })
    }

    pub fn apply_bonus(
        &mut self,
        board: &Board,
        mv: Move,
        prev: Option<(Piece, Move)>,
        depth: i16,
    ) {
        if let Some(entry) = self.get_mut(board, mv, prev) {
            apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(entry, Self::bonus(depth));
        }
    }

    pub fn apply_malus(
        &mut self,
        board: &Board,
        mv: Move,
        prev: Option<(Piece, Move)>,
        depth: i16,
    ) {
        if let Some(entry) = self.get_mut(board, mv, prev) {
            apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(entry, -Self::malus(depth));
        }
    }
}
