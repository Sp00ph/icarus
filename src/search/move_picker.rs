use arrayvec::ArrayVec;
use icarus_board::{board::Board, r#move::Move, movegen::Abort};

use crate::search::searcher::ThreadCtx;

#[derive(Clone, Copy, Debug)]
pub struct ScoredMove(pub Move, pub i16);

pub const MAX_MOVES: usize = 218;

pub type MoveList = ArrayVec<ScoredMove, MAX_MOVES>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    GenNoisy,
    YieldNoisy,
    GenQuiet,
    YieldQuiet,
}

pub struct MovePicker {
    moves: MoveList,
    index: usize,
    stage: Stage,
    skip_quiets: bool,
}

impl MovePicker {
    pub fn new(skip_quiets: bool) -> Self {
        Self {
            moves: MoveList::new(),
            index: 0,
            stage: Stage::GenNoisy,
            skip_quiets,
        }
    }

    pub fn next(&mut self, board: &Board, _: &ThreadCtx) -> Option<Move> {
        if self.stage == Stage::GenNoisy {
            board.gen_noisy_moves(|moves| {
                self.moves
                    .extend(moves.into_iter().map(|mv| ScoredMove(mv, 0)));
                Abort::No
            });
            for mv in &mut self.moves {
                let victim = 8 * mv.0.captures(board).map_or(0, |p| p.idx() + 1) as i16;
                let attacker = mv.0.piece_type(board) as i16;
                mv.1 = victim - attacker * i16::from(victim != 0);
            }

            self.stage = Stage::YieldNoisy;
        }

        'noisy: {
            if self.stage == Stage::YieldNoisy {
                if self.index >= self.moves.len() {
                    self.stage = Stage::GenQuiet;
                    break 'noisy;
                }

                let (i, mv) = self
                    .moves
                    .iter()
                    .copied()
                    .enumerate()
                    .skip(self.index)
                    .max_by_key(|(_, mv)| mv.1)
                    .unwrap();
                self.moves.swap(self.index, i);
                self.index += 1;
                return Some(mv.0);
            }
        }

        if self.stage == Stage::GenQuiet {
            self.moves.clear();
            self.index = 0;
            if !self.skip_quiets {
                board.gen_quiet_moves(|moves| {
                    self.moves
                        .extend(moves.into_iter().map(|mv| ScoredMove(mv, 0)));
                    Abort::No
                });
            }
            self.stage = Stage::YieldQuiet;
        }

        assert_eq!(self.stage, Stage::YieldQuiet);
        let mv = *self.moves.get(self.index)?;
        self.index += 1;
        Some(mv.0)
    }
}
