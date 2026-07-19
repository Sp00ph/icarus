use icarus_board::{board::Board, r#move::Move};

use crate::search::{
    history::{MAX_HIST_VALUE, apply_gravity},
    params::{
        tactic_bonus_base, tactic_bonus_max, tactic_bonus_scale, tactic_malus_base,
        tactic_malus_max, tactic_malus_scale,
    },
};

pub struct TacticHist {
    /// [stm][attacker][victim][to][from-threatened][to-threatened]
    data: [[[[[i16; 2]; 2]; 64]; 6]; 2],
}

impl TacticHist {
    fn bonus(depth: i16) -> i32 {
        (tactic_bonus_base() + (depth as i32) * tactic_bonus_scale()).min(tactic_bonus_max())
    }

    fn malus(depth: i16) -> i32 {
        (tactic_malus_base() + (depth as i32) * tactic_malus_scale()).min(tactic_malus_max())
    }

    pub fn get(&self, board: &Board, mv: Move) -> i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let piece = board.piece_on(from).unwrap();
        let from_threat = board.attacked().contains(from) as usize;
        let to_threat = board.attacked().contains(from) as usize;

        self.data[stm][piece][to][from_threat][to_threat]
    }

    fn get_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let piece = board.piece_on(from).unwrap();

        let from_threat = board.attacked().contains(from) as usize;
        let to_threat = board.attacked().contains(from) as usize;

        &mut self.data[stm][piece][to][from_threat][to_threat]
    }

    pub fn apply_bonus(&mut self, board: &Board, mv: Move, depth: i16) {
        apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(
            self.get_mut(board, mv),
            Self::bonus(depth),
        );
    }

    pub fn apply_malus(&mut self, board: &Board, mv: Move, depth: i16) {
        apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(
            self.get_mut(board, mv),
            -Self::malus(depth),
        );
    }
}
