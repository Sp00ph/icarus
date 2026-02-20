use icarus_board::r#move::Move;
use icarus_common::piece::{Color, Piece};

use crate::search::history::{MAX_CORR_VALUE, apply_gravity};

pub struct ContCorrHist {
    /// [stm][prev piece][prev dst][piece][dst]
    data: [[[[[i16; 64]; 6]; 64]; 6]; 2],
}

impl ContCorrHist {
    pub fn get(&self, stm: Color, cur: Option<(Piece, Move)>, prev: Option<(Piece, Move)>) -> i16 {
        cur.zip(prev)
            .map_or(0, |((cur_piece, cur_mv), (prev_piece, prev_mv))| {
                self.data[stm][prev_piece][prev_mv.to()][cur_piece][cur_mv.to()]
            })
    }

    fn get_mut(
        &mut self,
        stm: Color,
        cur: Option<(Piece, Move)>,
        prev: Option<(Piece, Move)>,
    ) -> Option<&mut i16> {
        cur.zip(prev)
            .map(|((cur_piece, cur_mv), (prev_piece, prev_mv))| {
                &mut self.data[stm][prev_piece][prev_mv.to()][cur_piece][cur_mv.to()]
            })
    }

    pub fn update(
        &mut self,
        stm: Color,
        cur: Option<(Piece, Move)>,
        prev: Option<(Piece, Move)>,
        amount: i32,
    ) {
        if let Some(entry) = self.get_mut(stm, cur, prev) {
            apply_gravity::<{ MAX_CORR_VALUE / 4 }, MAX_CORR_VALUE>(entry, amount);
        }
    }
}
