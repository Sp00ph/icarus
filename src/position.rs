use icarus_board::{
    attack_generators::{bishop_moves, rook_moves},
    board::{Board, TerminalState},
    r#move::{Move, MoveFlag},
};
use icarus_common::{
    lookups::{king_moves, knight_moves, pawn_attacks},
    piece::{Color, Piece},
    square::Square,
};

use crate::{nnue::network::Nnue, score::Score, weights::see_val};

#[derive(Clone)]
pub struct Position {
    board: Board,
    /// Previously played boards. `history[0]` is the starting position.
    history: Vec<Board>,
    moves: Vec<Option<(Piece, Move)>>,
}

impl Position {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            history: vec![],
            moves: vec![],
        }
    }

    pub fn make_move(&mut self, mv: Move, nnue: Option<&mut Nnue>) {
        if let Some(nnue) = nnue {
            nnue.make_move(&self.board, mv);
        }
        let piece = mv.piece_type(&self.board);
        self.history.push(self.board);
        self.board.make_move(mv);
        self.moves.push(Some((piece, mv)));
    }

    pub fn make_null_move(&mut self) {
        self.history.push(self.board);
        self.board.make_null_move();
        self.moves.push(None);
    }

    pub fn unmake_move(&mut self, nnue: Option<&mut Nnue>) {
        if let Some(nnue) = nnue {
            nnue.unmake_move();
        }
        self.board = self.history.pop().unwrap();
        self.moves.pop();
    }

    // Only here for completeness when I add NNUE :3
    pub fn unmake_null_move(&mut self) {
        self.board = self.history.pop().unwrap();
        self.moves.pop();
    }

    pub fn eval(&self, nnue: &mut Nnue) -> Score {
        nnue.update();
        let eval = nnue.eval(self.board.stm());
        Score::clamp_nomate(eval.clamp(i16::MIN as i32, i16::MAX as i32) as i16)
    }

    pub fn prev_move(&self, ply: usize) -> Option<(Piece, Move)> {
        self.moves
            .len()
            .checked_sub(ply)
            .and_then(|i| self.moves[i])
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn repetition(&self) -> bool {
        // It's important for codegen quality here that we skip(3).take(max(hm - 3, 0)) instead of take(hm).skip(3)
        self.history
            .iter()
            .rev()
            .skip(3)
            .take((self.board.halfmove_clock() as usize).saturating_sub(3))
            .step_by(2)
            .any(|b| b.hash() == self.board.hash())
    }

    pub fn is_draw(&self) -> bool {
        self.board.terminal_state() == Some(TerminalState::Draw) || self.repetition()
    }

    pub fn cmp_see(&self, mv: Move, threshold: i16) -> bool {
        // Heavily inspired by <https://github.com/AndyGrant/Ethereal/blob/0e47e9b67f345c75eb965d9fb3e2493b6a11d09a/src/search.c>

        let board = &self.board;
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        if flag == MoveFlag::Castle {
            return threshold <= 0;
        }

        let next_victim = mv
            .promotes_to()
            .or_else(|| board.piece_on(mv.from()))
            .unwrap();

        let mut balance = -threshold
            + mv.captures(board).map_or(0, see_val)
            + mv.promotes_to()
                .map_or(0, |promo| see_val(promo) - see_val(Piece::Pawn));
        if balance < 0 {
            return false;
        }

        balance -= see_val(next_victim);

        if balance >= 0 {
            return true;
        }

        let orth = board.pieces(Piece::Rook) | board.pieces(Piece::Queen);
        let diag = board.pieces(Piece::Bishop) | board.pieces(Piece::Queen);

        let mut occupied = board.occupied() ^ from | to;
        if flag == MoveFlag::EnPassant {
            occupied ^= Square::new(to.file(), from.rank());
        }

        #[rustfmt::skip]
        let mut attackers = (
            (pawn_attacks(to, Color::White) & board.occupied_by(Color::Black) & board.pieces(Piece::Pawn))
            | (pawn_attacks(to, Color::Black) & board.occupied_by(Color::White) & board.pieces(Piece::Pawn))
            | (knight_moves(to) & board.pieces(Piece::Knight))
            | (bishop_moves(to, occupied) & diag)
            | (rook_moves(to, occupied) & orth)
            | (king_moves(to) & board.pieces(Piece::King))
        ) & occupied;

        let mut stm = !board.stm();

        loop {
            let my_attackers = attackers & board.occupied_by(stm);
            if my_attackers.is_empty() {
                break;
            }

            let next_victim = Piece::all()
                .find(|&p| (my_attackers & board.pieces(p)).is_non_empty())
                .unwrap();

            occupied ^= (board.pieces(next_victim) & my_attackers).next();

            if [Piece::Pawn, Piece::Bishop, Piece::Queen].contains(&next_victim) {
                attackers |= bishop_moves(to, occupied) & diag;
            }

            if [Piece::Rook, Piece::Queen].contains(&next_victim) {
                attackers |= rook_moves(to, occupied) & orth;
            }

            attackers &= occupied;
            stm = !stm;

            balance = -balance - 1 - see_val(next_victim);

            if balance >= 0 {
                if next_victim == Piece::King && (attackers & board.occupied_by(stm)).is_non_empty()
                {
                    stm = !stm;
                }
                break;
            }
        }

        board.stm() != stm
    }
}
