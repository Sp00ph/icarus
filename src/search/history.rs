use icarus_board::{board::Board, r#move::Move};

use crate::score::Score;

const MAX_CORR_VALUE: i32 = 1024;
const MAX_HIST_VALUE: i32 = 16384;

const PAWN_CORR_SIZE: usize = 16384;

pub struct History {
    /// [stm][from][from attacked][to][to attacked]
    quiet: [[[[[i16; 2]; 64]; 2]; 64]; 2],
    /// [stm][pawn hash % PAWN_CORR_SIZE]
    pawn_corr: [[i16; PAWN_CORR_SIZE]; 2],
}

impl Default for History {
    fn default() -> Self {
        Self {
            quiet: [[[[[0; 2]; 64]; 2]; 64]; 2],
            pawn_corr: [[0; PAWN_CORR_SIZE]; 2],
        }
    }
}

impl History {
    pub fn score_quiet(&self, board: &Board, mv: Move) -> i16 {
        self.quiet[board.stm()][mv.from()][board.attacked().contains(mv.from()) as usize][mv.to()]
            [board.attacked().contains(mv.to()) as usize]
    }

    pub fn corr(&self, board: &Board) -> i16 {
        let pawn_factor = 64;
        ((self.pawn_corr[board.stm()][board.pawn_hash() as usize % PAWN_CORR_SIZE] as i32)
            * pawn_factor
            / MAX_CORR_VALUE) as i16
    }

    fn quiet_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        &mut self.quiet[board.stm()][mv.from()][board.attacked().contains(mv.from()) as usize]
            [mv.to()][board.attacked().contains(mv.to()) as usize]
    }

    pub fn update(&mut self, board: &Board, mv: Move, quiets: &[Move], depth: i16) {
        if board.is_tactic(mv) {
            return;
        }

        let bonus_base = 128;
        let bonus_scale = 128;
        let bonus_max = 2048;
        let bonus = (bonus_base + (depth as i32) * bonus_scale).min(bonus_max);

        let malus_base = 128;
        let malus_scale = 128;
        let malus_max = 2048;
        let malus = (malus_base + (depth as i32) * malus_scale).min(malus_max);

        Self::update_value(self.quiet_mut(board, mv), bonus);

        for &quiet in quiets {
            Self::update_value(self.quiet_mut(board, quiet), -malus);
        }
    }

    pub fn update_corr(&mut self, board: &Board, depth: i16, score: Score, static_eval: Score) {
        let bonus_scale = 128;

        let delta = score.0 as i32 - static_eval.0 as i32;
        let amount = (delta * (depth as i32) * bonus_scale) / 1024;

        Self::update_corr_val(
            &mut self.pawn_corr[board.stm()][board.pawn_hash() as usize % PAWN_CORR_SIZE],
            amount,
        );
    }

    fn update_value(value: &mut i16, amount: i32) {
        let amount = amount.clamp(-MAX_HIST_VALUE, MAX_HIST_VALUE);
        let decay = (*value as i32 * amount.abs() / MAX_HIST_VALUE) as i16;

        *value += amount as i16 - decay;
    }

    fn update_corr_val(value: &mut i16, amount: i32) {
        let amount = amount.clamp(-MAX_CORR_VALUE / 4, MAX_CORR_VALUE / 4);
        let decay = (*value as i32 * amount.abs() / MAX_CORR_VALUE) as i16;

        *value += amount as i16 - decay;
    }
}
