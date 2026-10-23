use crate::{
    board::{
        Board,
        r#move::{Move, MoveFlag},
        sliders::{bishop_moves, rook_moves},
        zobrist::ZOBRIST,
    },
    common::{
        lookups::{between, king_moves, knight_moves, pawn_attacks},
        piece::{Color, Piece},
        square::Square,
    },
    nnue::network::Nnue,
    score::Score,
    search::{
        cuckoo::{self, Cuckoo, CuckooTag},
        params::{mat_scale, mat_scaling_base, see_val},
    },
};

#[derive(Clone)]
pub struct Position {
    board: Board,
    /// Previously played boards. `history[0]` is the starting position.
    history: Vec<Board>,
    moves: Vec<Option<(Piece, Move)>>,

    cuckoo_tag: CuckooTag,
}

impl Position {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            history: vec![],
            moves: vec![],

            cuckoo_tag: Cuckoo::init(),
        }
    }

    pub fn make_move(&mut self, mv: Move, nnue: Option<&mut Nnue>) {
        let piece = mv.piece_type(&self.board);
        if let Some(nnue) = nnue {
            nnue.make_move(&self.board, mv);
        }
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

    pub fn unmake_null_move(&mut self) {
        self.board = self.history.pop().unwrap();
        self.moves.pop();
    }

    pub fn eval(&self, nnue: &mut Nnue, mat_scaling: bool) -> Score {
        nnue.update(&self.board);
        let eval = nnue.eval(self.board.stm());

        let scale = if mat_scaling {
            mat_scaling_base()
                + Piece::all()
                    .map(|pt| self.board.pieces(pt).popcnt() as i32 * mat_scale(pt))
                    .sum::<i32>()
        } else {
            32768
        };

        Score::clamp_nomate((eval * scale / 32768).clamp(i16::MIN as i32, i16::MAX as i32) as i16)
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

    pub fn repetition(&self, ply: u16) -> bool {
        let mut matches = self
            .history
            .iter()
            .rev()
            .enumerate()
            .take(self.board.halfmove_clock() as usize)
            .skip(3)
            .step_by(2)
            .filter(|(_, b)| b.hash() == self.board.hash());

        // Two-fold counts as a repetition if both positions occurred after root. Otherwise
        // we check for threefold.
        matches.next().is_some_and(|(i, _)| i < ply as usize) || matches.next().is_some()
    }

    pub fn upcoming_repetition(&self, ply: u16) -> bool {
        let end = self.history.len().min(self.board.halfmove_clock as usize);

        if end < 3 {
            return false;
        }

        let old_key = |i: usize| self.history[self.history.len() - i].hash();

        let occ = self.board.occupied();
        let orig_key = self.board.hash();

        let mut other = orig_key ^ old_key(1) ^ ZOBRIST.black_to_move;

        for i in (3..=end).step_by(2) {
            let curr_key = old_key(i);

            other ^= curr_key ^ old_key(i - 1) ^ ZOBRIST.black_to_move;
            if other != 0 {
                continue;
            }

            let diff = orig_key ^ curr_key;

            let cuckoo = Cuckoo::get(self.cuckoo_tag);
            let mut slot = cuckoo::h1(diff);
            if diff != cuckoo.keys[slot] {
                slot = cuckoo::h2(diff)
            }
            if diff != cuckoo.keys[slot] {
                continue;
            }

            let (sq1, sq2) = cuckoo.moves[slot];

            if (between(sq1, sq2) & occ).is_non_empty() {
                continue;
            }

            // twofold before root
            if i <= ply as usize {
                return true;
            }

            // threefold beyond root
            if (i + 2..=end).step_by(2).any(|j| old_key(j) == curr_key) {
                return true;
            }
        }

        false
    }

    pub fn cmp_see(&self, mv: Move, threshold: i32) -> bool {
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
            | (bishop_moves(to, occupied, self.board.slider_tag) & diag)
            | (rook_moves(to, occupied, self.board.slider_tag) & orth)
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
                attackers |= bishop_moves(to, occupied, self.board.slider_tag) & diag;
            }

            if [Piece::Rook, Piece::Queen].contains(&next_victim) {
                attackers |= rook_moves(to, occupied, self.board.slider_tag) & orth;
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
