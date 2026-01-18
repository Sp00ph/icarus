use arrayvec::ArrayVec;
use icarus_board::{r#move::Move, movegen::Abort};
use icarus_common::piece::Piece;

use crate::{position::Position, search::searcher::ThreadCtx, weights::see_val};

#[derive(Clone, Copy, Debug)]
pub struct ScoredMove(pub Move, pub i16);

pub const MAX_MOVES: usize = 218;

pub type MoveList = ArrayVec<ScoredMove, MAX_MOVES>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Stage {
    TTMove,
    KillerMove,
    GenNoisy,
    YieldGoodNoisy,
    GenQuiet,
    YieldQuiet,
    YieldBadNoisy,
}

pub struct MovePicker {
    moves: MoveList,
    bad_noisies: usize,
    index: usize,
    stage: Stage,
    skip_quiets: bool,
    tt_move: Option<Move>,
    killer_move: Option<Move>,
    see_threshold: i16,
    skip_bad_noisies: bool,
}

impl MovePicker {
    pub fn new(
        tt_move: Option<Move>,
        // Note: We assume that killer_move, if given, is quiet.
        killer_move: Option<Move>,
        skip_quiets: bool,
        see_threshold: i16,
        skip_bad_noisies: bool,
    ) -> Self {
        let killer_move = if killer_move != tt_move {
            killer_move
        } else {
            None
        };
        Self {
            moves: MoveList::new(),
            bad_noisies: 0,
            index: 0,
            stage: Stage::TTMove,
            skip_quiets,
            tt_move,
            killer_move,
            see_threshold,
            skip_bad_noisies,
        }
    }

    pub fn skip_quiets(&mut self) {
        self.skip_quiets = true;
        if [Stage::GenQuiet, Stage::YieldQuiet].contains(&self.stage) {
            self.index = 0;
            self.stage = Stage::YieldBadNoisy;
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn no_more_quiets(&self) -> bool {
        self.skip_quiets || self.stage > Stage::YieldQuiet
    }

    fn pick_best(&self) -> (usize, Move) {
        let packed = self
            .moves
            .iter()
            .enumerate()
            .skip(self.index)
            .map(|(i, mv)| (i as i32) | (mv.1 as i32) << 16)
            .fold(i32::MIN, std::cmp::max);
        let idx = packed as usize & 0xffff;
        (idx, self.moves[idx].0)
    }

    pub fn next(&mut self, pos: &Position, thread: &ThreadCtx) -> Option<Move> {
        let board = pos.board();

        if self.stage == Stage::TTMove {
            self.stage = Stage::GenNoisy;
            if let Some(mv) = self.tt_move
                && board.is_legal(mv)
            {
                return Some(mv);
            }
        }

        if self.stage == Stage::GenNoisy {
            board.gen_noisy_moves(|moves| {
                self.moves.extend(
                    moves
                        .into_iter()
                        // don't need to filter out the killer here, as it's quiet.
                        .filter(|mv| self.tt_move != Some(*mv))
                        .map(|mv| ScoredMove(mv, 0)),
                );
                Abort::No
            });
            for mv in &mut self.moves {
                let victim = mv.0.captures(board).map_or(0, see_val);
                mv.1 = thread.history.score_tactic(board, mv.0) / 32 + victim * 8;
                if let Some(promo) = mv.0.promotes_to() {
                    mv.1 += (see_val(promo) - see_val(Piece::Pawn)) * 8;
                }
            }

            self.stage = Stage::YieldGoodNoisy;
        }

        while self.stage == Stage::YieldGoodNoisy {
            if self.index == self.moves.len() {
                self.stage = Stage::KillerMove;
                break;
            }

            let (i, mv) = self.pick_best();
            self.moves.swap(self.index, i);
            self.index += 1;

            if pos.cmp_see(mv, self.see_threshold) {
                return Some(mv);
            }

            self.moves.swap(self.bad_noisies, self.index - 1);
            self.bad_noisies += 1;
        }

        if self.stage == Stage::KillerMove {
            self.stage = Stage::GenQuiet;
            if !self.skip_quiets
                && let Some(mv) = self.killer_move
                && board.is_legal(mv)
            {
                return Some(mv);
            }
        }

        if self.stage == Stage::GenQuiet {
            if !self.skip_quiets {
                board.gen_quiet_moves(|moves| {
                    self.moves.extend(
                        moves
                            .into_iter()
                            .filter(|mv| ![self.tt_move, self.killer_move].contains(&Some(*mv)))
                            .map(|mv| ScoredMove(mv, thread.history.score_quiet(pos, mv))),
                    );
                    Abort::No
                });
            }
            self.stage = Stage::YieldQuiet;
        }

        'quiet: {
            if self.stage == Stage::YieldQuiet {
                if self.index == self.moves.len() {
                    self.index = 0;
                    self.stage = Stage::YieldBadNoisy;
                    break 'quiet;
                }

                let (i, mv) = self.pick_best();
                self.moves.swap(self.index, i);
                self.index += 1;
                return Some(mv);
            }
        }

        assert_eq!(self.stage, Stage::YieldBadNoisy);
        if self.skip_bad_noisies || self.index >= self.bad_noisies {
            return None;
        }
        let mv = self.moves[self.index].0;
        self.index += 1;
        Some(mv)
    }
}
