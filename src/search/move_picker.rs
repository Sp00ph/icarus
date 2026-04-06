use arrayvec::ArrayVec;
use icarus_board::{
    attack_generators::{bishop_moves, rook_moves},
    r#move::Move,
    movegen::Abort,
};
use icarus_common::{
    bitboard::Bitboard,
    direction::{DownLeft, DownRight},
    lookups::knight_moves,
    piece::Piece,
    util::enum_map::enum_map,
};

use crate::{position::Position, search::params::see_val, search::searcher::ThreadCtx};

#[derive(Clone, Copy, Debug)]
pub struct ScoredMove(pub Move, pub i32);

pub const MAX_MOVES: usize = 218;

pub type MoveList = ArrayVec<ScoredMove, MAX_MOVES>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Stage {
    TTMove,
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
    see_threshold: i32,
}

impl MovePicker {
    pub fn new(tt_move: Option<Move>, skip_quiets: bool, see_threshold: i32) -> Self {
        Self {
            moves: MoveList::new(),
            bad_noisies: 0,
            index: 0,
            stage: Stage::TTMove,
            skip_quiets,
            tt_move,
            see_threshold,
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
            .map(|(i, mv)| (i as i64) | (mv.1 as i64) << 32)
            .fold(i64::MIN, std::cmp::max);
        let idx = packed as usize & 0xffffffff;
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
                        .filter(|mv| self.tt_move != Some(*mv))
                        .map(|mv| ScoredMove(mv, 0)),
                );
                Abort::No
            });
            for mv in &mut self.moves {
                let victim = mv.0.captures(board).map_or(0, see_val);
                mv.1 = thread.history.score_tactic(board, mv.0) / 8 + victim * 8;
                if let Some(promo) = mv.0.promotes_to() {
                    mv.1 += (see_val(promo) - see_val(Piece::Pawn)) * 8;
                }
            }

            self.stage = Stage::YieldGoodNoisy;
        }

        while self.stage == Stage::YieldGoodNoisy {
            if self.index == self.moves.len() {
                self.stage = Stage::GenQuiet;
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

        if self.stage == Stage::GenQuiet {
            if !self.skip_quiets {
                let threats = board.attacked();
                let stm = board.stm();
                let [mut n, mut b, mut q] = [Bitboard::EMPTY; 3];
                let pawn_offense = (board.occupied_by(!stm).shift::<DownLeft>(stm.signum())
                    | board.occupied_by(!stm).shift::<DownRight>(stm.signum()))
                    & !threats;

                for sq in board.colored_pieces(Piece::Bishop, !stm) & !threats {
                    n |= knight_moves(sq);
                    q |= rook_moves(sq, board.occupied());
                }

                for sq in board.colored_pieces(Piece::Rook, !stm) {
                    let bishop_moves = bishop_moves(sq, board.occupied());
                    n |= knight_moves(sq);
                    b |= bishop_moves;
                    if !threats.contains(sq) {
                        q |= bishop_moves;
                    }
                }

                for sq in board.colored_pieces(Piece::Queen, !stm) {
                    n |= knight_moves(sq);
                }

                let offense = enum_map! {
                    Piece::Pawn => pawn_offense,
                    Piece::Knight => n & !threats,
                    Piece::Bishop => b & !threats,
                    Piece::Rook => Bitboard::EMPTY,
                    Piece::Queen => q & !threats,
                    Piece::King => Bitboard::EMPTY
                };

                board.gen_quiet_moves(|moves| {
                    self.moves.extend(
                        moves
                            .into_iter()
                            .filter(|mv| self.tt_move != Some(*mv))
                            .map(|mv| {
                                let pt = board.piece_on(mv.from()).unwrap();
                                ScoredMove(
                                    mv,
                                    thread.history.score_quiet(pos, mv)
                                        + 8000 * pos.board().gives_direct_check(mv) as i32
                                        + 6000 * offense[pt].contains(mv.to()) as i32,
                                )
                            }),
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
        if self.index >= self.bad_noisies {
            return None;
        }
        let mv = self.moves[self.index].0;
        self.index += 1;
        Some(mv)
    }
}
