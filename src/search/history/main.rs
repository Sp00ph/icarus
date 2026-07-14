use icarus_board::{board::Board, r#move::Move};

use crate::search::{
    history::{MAX_HIST_VALUE, apply_gravity},
    params::{
        main_bonus_base, main_bonus_max, main_bonus_scale, main_malus_base, main_malus_max,
        main_malus_scale,
    },
};

pub struct MainHist {
    /// [stm][from][from attacked][to][to attacked]
    data: [[[[[i16; 2]; 64]; 2]; 64]; 2],
}

impl MainHist {
    fn bonus(depth: i16) -> i32 {
        (main_bonus_base() + (depth as i32) * main_bonus_scale()).min(main_bonus_max())
    }

    fn malus(depth: i16) -> i32 {
        (main_malus_base() + (depth as i32) * main_malus_scale()).min(main_malus_max())
    }

    pub fn get(&self, board: &Board, mv: Move) -> i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let from_threatened = board.attacked().contains(from) as usize;
        let to_threatened = board.attacked().contains(to) as usize;

        self.data[stm][from][from_threatened][to][to_threatened]
    }

    fn get_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let from_threatened = board.attacked().contains(from) as usize;
        let to_threatened = board.attacked().contains(to) as usize;

        &mut self.data[stm][from][from_threatened][to][to_threatened]
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
