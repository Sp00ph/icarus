pub mod cont;
pub mod contcorr;
pub mod corr;
pub mod main;
pub mod tactic;

use icarus_board::{board::Board, r#move::Move};
use icarus_common::piece::Color;

use crate::{
    position::Position,
    score::Score,
    search::{
        history::{
            cont::ContHist, contcorr::ContCorrHist, corr::CorrHist, main::MainHist,
            tactic::TacticHist,
        },
        params::{
            corr_black_factor, corr_cont1_factor, corr_cont2_factor, corr_major_factor,
            corr_minor_factor, corr_pawn_factor, corr_white_factor,
        },
    },
};

const MAX_CORR_VALUE: i32 = 1024;
const MAX_HIST_VALUE: i32 = 16384;

const CORR_SIZE: usize = 16384;

pub struct History {
    main: MainHist,
    tactic: TacticHist,
    cont_oneply: ContHist<1>,
    cont_twoply: ContHist<2>,

    pawn_corr: CorrHist,
    minor_corr: CorrHist,
    major_corr: CorrHist,
    white_nonpawn_corr: CorrHist,
    black_nonpawn_corr: CorrHist,

    contcorr_oneply: ContCorrHist,
    contcorr_twoply: ContCorrHist,
}

fn apply_gravity<const MAX_BONUS: i32, const MAX_VALUE: i32>(
    entry: &mut i16,
    total: Option<i16>,
    amount: i32,
) {
    let amount = amount.clamp(-MAX_BONUS, MAX_BONUS);
    let decay = (total.unwrap_or(*entry) as i32 * amount.abs() / MAX_VALUE) as i16;
    *entry += amount as i16 - decay;
}

impl History {
    pub fn new() -> Box<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    pub fn clear(&mut self) {
        unsafe { std::ptr::write_bytes(self, 0, 1) }
    }

    pub fn score_quiet(&self, pos: &Position, mv: Move) -> i16 {
        let board = pos.board();
        let oneply = pos.prev_move(1);
        let twoply = pos.prev_move(2);
        self.main
            .get(pos.board(), mv)
            .saturating_add(self.cont_oneply.get(board, mv, oneply))
            .saturating_add(self.cont_twoply.get(board, mv, twoply))
    }

    pub fn score_tactic(&self, board: &Board, mv: Move) -> i16 {
        self.tactic.get(board, mv)
    }

    pub fn corr(&self, pos: &Position) -> i16 {
        let board = pos.board();
        let stm = board.stm();
        let (twoply, oneply, cur) = (pos.prev_move(3), pos.prev_move(2), pos.prev_move(1));

        let mut corr = 0;
        corr += (self.pawn_corr.get(stm, board.pawn_hash()) as i32) * corr_pawn_factor();
        corr += (self.minor_corr.get(stm, board.minor_hash()) as i32) * corr_minor_factor();
        corr += (self.major_corr.get(stm, board.major_hash()) as i32) * corr_major_factor();
        corr += (self
            .white_nonpawn_corr
            .get(stm, board.nonpawn_hash(Color::White)) as i32)
            * corr_white_factor();
        corr += (self
            .black_nonpawn_corr
            .get(stm, board.nonpawn_hash(Color::Black)) as i32)
            * corr_black_factor();

        corr += self.contcorr_oneply.get(stm, cur, oneply) as i32 * corr_cont1_factor();
        corr += self.contcorr_twoply.get(stm, cur, twoply) as i32 * corr_cont2_factor();

        (corr / MAX_CORR_VALUE) as i16
    }

    pub fn update(
        &mut self,
        pos: &Position,
        mv: Move,
        quiets: &[Move],
        tactics: &[Move],
        depth: i16,
    ) {
        let board = pos.board();
        let oneply = pos.prev_move(1);
        let twoply = pos.prev_move(2);
        let cont_score = self
            .cont_oneply
            .get(board, mv, oneply)
            .saturating_add(self.cont_twoply.get(board, mv, twoply));

        if board.is_tactic(mv) {
            self.tactic.apply_bonus(board, mv, depth);
        } else {
            self.main.apply_bonus(board, mv, depth);
            self.cont_oneply
                .apply_bonus(board, mv, oneply, cont_score, depth);
            self.cont_twoply
                .apply_bonus(board, mv, twoply, cont_score, depth);

            for &quiet in quiets {
                self.main.apply_malus(board, quiet, depth);
                self.cont_oneply
                    .apply_malus(board, quiet, oneply, cont_score, depth);
                self.cont_twoply
                    .apply_malus(board, quiet, twoply, cont_score, depth);
            }
        }

        for &tactic in tactics {
            self.tactic.apply_malus(board, tactic, depth);
        }
    }

    pub fn update_corr(&mut self, pos: &Position, depth: i16, score: Score, static_eval: Score) {
        let board = pos.board();
        let stm = board.stm();
        let (twoply, oneply, cur) = (pos.prev_move(3), pos.prev_move(2), pos.prev_move(1));

        let total = self
            .contcorr_oneply
            .get(stm, cur, oneply)
            .saturating_add(self.contcorr_twoply.get(stm, cur, twoply));

        let delta = score.0 as i32 - static_eval.0 as i32;

        let amount = CorrHist::amount(delta, depth);

        self.pawn_corr.update(stm, board.pawn_hash(), amount);
        self.minor_corr.update(stm, board.minor_hash(), amount);
        self.major_corr.update(stm, board.major_hash(), amount);
        self.white_nonpawn_corr
            .update(stm, board.nonpawn_hash(Color::White), amount);
        self.black_nonpawn_corr
            .update(stm, board.nonpawn_hash(Color::Black), amount);

        self.contcorr_oneply.update(stm, cur, oneply, total, amount);
        self.contcorr_twoply.update(stm, cur, twoply, total, amount);
    }
}
