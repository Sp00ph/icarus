use icarus_board::{board::Board, r#move::Move};

use crate::search::history::{MAX_HIST_VALUE, apply_gravity};

pub struct TacticHist {
    /// [stm][attacker][victim][to]
    data: [[[i16; 64]; 6]; 2],
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
        let piece = board.piece_on(from).unwrap();

        self.data[stm][piece][to]
    }

    fn get_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let piece = board.piece_on(from).unwrap();

        &mut self.data[stm][piece][to]
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
