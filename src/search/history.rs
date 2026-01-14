use icarus_board::{board::Board, r#move::Move};

const MAX_HISTORY: i32 = 16384;

pub struct History {
    /// [stm][from][from attacked][to][to attacked]
    quiet: [[[[[i16; 2]; 64]; 2]; 64]; 2],
}

impl Default for History {
    fn default() -> Self {
        Self {
            quiet: [[[[[0; 2]; 64]; 2]; 64]; 2],
        }
    }
}

impl History {
    pub fn score_quiet(&self, board: &Board, mv: Move) -> i16 {
        self.quiet[board.stm()][mv.from()][board.attacked().contains(mv.from()) as usize][mv.to()]
            [board.attacked().contains(mv.to()) as usize]
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

    fn update_value(value: &mut i16, amount: i32) {
        let amount = amount.clamp(-MAX_HISTORY, MAX_HISTORY);
        let decay = (*value as i32 * amount.abs() / MAX_HISTORY) as i16;

        *value += amount as i16 - decay;
    }
}
