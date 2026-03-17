use icarus_board::{board::Board, r#move::Move};
use icarus_common::piece::Piece;

use crate::search::{
    history::MAX_HIST_VALUE,
    params::{
        cont1_bonus_base, cont1_bonus_max, cont1_bonus_scale, cont1_malus_base, cont1_malus_max,
        cont1_malus_scale, cont2_bonus_base, cont2_bonus_max, cont2_bonus_scale, cont2_malus_base,
        cont2_malus_max, cont2_malus_scale,
    },
};

pub struct ContHist<const PLY: usize> {
    /// [stm][prev piece][prev dst][piece][dst]
    data: [[[[[i16; 64]; 6]; 64]; 6]; 2],
}


fn apply_gravity<const MAX_BONUS: i32, const MAX_VALUE: i32>(entry: &mut i16, total: i16, amount: i32) {
    let amount = amount.clamp(-MAX_BONUS, MAX_BONUS);
    let decay = (total as i32 * amount.abs() / MAX_VALUE) as i16;
    *entry += amount as i16 - decay;
}

impl<const PLY: usize> ContHist<PLY> {
    fn bonus(depth: i16) -> i32 {
        let (bonus_base, bonus_scale, bonus_max) = match PLY {
            1 => (cont1_bonus_base(), cont1_bonus_scale(), cont1_bonus_max()),
            2 => (cont2_bonus_base(), cont2_bonus_scale(), cont2_bonus_max()),
            _ => unreachable!(),
        };
        (bonus_base + (depth as i32) * bonus_scale).min(bonus_max)
    }

    fn malus(depth: i16) -> i32 {
        let (malus_base, malus_scale, malus_max) = match PLY {
            1 => (cont1_malus_base(), cont1_malus_scale(), cont1_malus_max()),
            2 => (cont2_malus_base(), cont2_malus_scale(), cont2_malus_max()),
            _ => unreachable!(),
        };
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
        total: i16,
        depth: i16,
    ) {
        if let Some(entry) = self.get_mut(board, mv, prev) {
            apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(entry, total, Self::bonus(depth));
        }
    }

    pub fn apply_malus(
        &mut self,
        board: &Board,
        mv: Move,
        prev: Option<(Piece, Move)>,
        total: i16,
        depth: i16,
    ) {
        if let Some(entry) = self.get_mut(board, mv, prev) {
            apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(entry, total, -Self::malus(depth));
        }
    }
}
