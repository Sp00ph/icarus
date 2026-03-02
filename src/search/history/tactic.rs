use icarus_board::{board::Board, r#move::Move};

use crate::search::history::{MAX_HIST_VALUE, apply_gravity};

pub struct TacticHist {
    /// [stm][from][from attacked][to][to attacked]
    data: [[[[[i16; 2]; 64]; 2]; 64]; 2],
}

impl TacticHist {
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

    pub fn get(&self, board: &Board, mv: Move) -> i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let from_attacked = board.attacked().contains(from);
        let to_attacked = board.attacked().contains(from);

        self.data[stm][from][from_attacked as usize][to][to_attacked as usize]
    }

    fn get_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let from_attacked = board.attacked().contains(from);
        let to_attacked = board.attacked().contains(from);

        &mut self.data[stm][from][from_attacked as usize][to][to_attacked as usize]
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
