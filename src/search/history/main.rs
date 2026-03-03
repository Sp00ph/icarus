use icarus_board::{board::Board, r#move::Move};

use crate::search::history::{MAX_HIST_VALUE, apply_gravity};

pub struct MainHist {
    /// [stm][from][from attacked][to][to attacked]
    from_to: [[[[[i16; 2]; 64]; 2]; 64]; 2],
    /// [stm][piece][from attacked][to][to attacked]
    piece_to: [[[[[i16; 2]; 64]; 2]; 6]; 2],
}

impl MainHist {
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
        self.get_from_to(board, mv)
            .midpoint(self.get_piece_to(board, mv))
    }

    fn get_from_to(&self, board: &Board, mv: Move) -> i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let from_threatened = board.attacked().contains(from) as usize;
        let to_threatened = board.attacked().contains(to) as usize;

        self.from_to[stm][from][from_threatened][to][to_threatened]
    }

    fn get_from_to_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let from_threatened = board.attacked().contains(from) as usize;
        let to_threatened = board.attacked().contains(to) as usize;

        &mut self.from_to[stm][from][from_threatened][to][to_threatened]
    }

    fn get_piece_to(&self, board: &Board, mv: Move) -> i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let piece = board.piece_on(from).unwrap();
        let from_threatened = board.attacked().contains(from) as usize;
        let to_threatened = board.attacked().contains(to) as usize;

        self.piece_to[stm][piece][from_threatened][to][to_threatened]
    }

    fn get_piece_to_mut(&mut self, board: &Board, mv: Move) -> &mut i16 {
        let (stm, from, to) = (board.stm(), mv.from(), mv.to());
        let piece = board.piece_on(from).unwrap();
        let from_threatened = board.attacked().contains(from) as usize;
        let to_threatened = board.attacked().contains(to) as usize;

        &mut self.piece_to[stm][piece][from_threatened][to][to_threatened]
    }

    pub fn apply_bonus(&mut self, board: &Board, mv: Move, depth: i16) {
        apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(
            self.get_from_to_mut(board, mv),
            Self::bonus(depth),
        );
        apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(
            self.get_piece_to_mut(board, mv),
            Self::bonus(depth),
        );
    }

    pub fn apply_malus(&mut self, board: &Board, mv: Move, depth: i16) {
        apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(
            self.get_from_to_mut(board, mv),
            -Self::malus(depth),
        );
        apply_gravity::<MAX_HIST_VALUE, MAX_HIST_VALUE>(
            self.get_piece_to_mut(board, mv),
            -Self::malus(depth),
        );
    }
}
