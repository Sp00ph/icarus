
#[cfg(feature = "test-islegal")] pub mod test;

use icarus_common::{
    lookups::{
        between, between_inclusive, king_moves, knight_moves, line, pawn_attacks, pawn_pushes,
    },
    piece::Piece,
    square::{Rank, Square},
};

use crate::{
    attack_generators::{bishop_moves, rook_moves},
    board::Board,
    castling::CastlingDirection,
    r#move::{Move, MoveFlag},
};

impl Board {
    /// Returns whether the move `mv` is legal to make in the current position.
    /// Crucially, it is assumed that there is at least one possible position in
    /// which `mv` is legal; so it is assumed that e.g. the promotion flag is only
    /// set if the move moves to rank 1 or rank 8.
    #[inline]
    pub fn is_legal(&self, mv: Move) -> bool {
        match self.checkers.popcnt() {
            0 => self.is_legal_no_check(mv),
            1 => self.is_legal_check(mv),
            _ => self.is_legal_evasion(mv),
        }
    }

    fn is_legal_no_check(&self, mv: Move) -> bool {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        let Some(piece) = self.colored_piece_on(from, self.stm) else {
            return false;
        };

        let our_king = self.king(self.stm);
        let blockers = self.occupied();
        let targets = !self.occupied_by(self.stm);

        match flag {
            MoveFlag::Castle => return self.is_legal_castle(mv),
            MoveFlag::EnPassant => return self.is_legal_en_passant(mv),
            MoveFlag::Promotion if piece != Piece::Pawn => return false,
            _ => {}
        };

        // The only move that could "capture" a friendly piece is castles, which is handled above.
        if !targets.contains(to) {
            return false;
        }

        // If we are pinned, and we leave the line along which we are pinned, it would always be a discovered check.
        if self.pinned.contains(from) && !line(our_king, from).contains(to) {
            return false;
        }

        match piece {
            Piece::Pawn => {
                (to.rank() != Rank::R8.relative_to(self.stm) || flag == MoveFlag::Promotion)
                    && (((pawn_attacks(from, self.stm) & self.occupied_by(!self.stm)).contains(to))
                        || pawn_pushes(from, self.stm, blockers).contains(to))
            }
            Piece::Knight => knight_moves(from).contains(to),
            Piece::Bishop => bishop_moves(from, blockers).contains(to),
            Piece::Rook => rook_moves(from, blockers).contains(to),
            Piece::Queen => {
                (bishop_moves(from, blockers) | rook_moves(from, blockers)).contains(to)
            }
            Piece::King => (king_moves(from) & !self.attacked).contains(to),
        }
    }

    fn is_legal_check(&self, mv: Move) -> bool {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        let piece = match self.colored_piece_on(from, self.stm) {
            None => return false,
            Some(Piece::King) => return self.is_legal_evasion(mv),
            Some(piece) => piece,
        };

        match flag {
            MoveFlag::Promotion => {
                if piece != Piece::Pawn || to.rank() != Rank::R8.relative_to(self.stm) {
                    return false;
                }
            }
            MoveFlag::Castle => return false,
            // en passant is handled below
            _ => {}
        }

        let checker = self.checkers.next();
        let our_king = self.king(self.stm);
        let targets = between(our_king, checker) | checker;
        let blockers = self.occupied();

        if !targets.contains(to) {
            // The only legal move when in check that doesnt land on the attacker or between it and
            // our king is en passant. And since a double pawn push can never lead to a double check,
            // en passant is legal iff the checker is the double pushed pawn.
            return self.en_passant.is_some_and(|ep| {
                checker == Square::new(ep.file(), Rank::R5.relative_to(self.stm))
            }) && self.is_legal_en_passant(mv);
        }

        // The above case handled all en passant possibilities.
        if flag == MoveFlag::EnPassant {
            return false;
        }

        // If we are pinned, and we leave the line along which we are pinned, it would always be a discovered check.
        if self.pinned.contains(from) && !line(our_king, from).contains(to) {
            return false;
        }

        match piece {
            Piece::Pawn => {
                (to.rank() != Rank::R8.relative_to(self.stm) || flag == MoveFlag::Promotion)
                    && (((pawn_attacks(from, self.stm) & checker).contains(to))
                        || pawn_pushes(from, self.stm, blockers).contains(to))
            }
            Piece::Knight => knight_moves(from).contains(to),
            Piece::Bishop => bishop_moves(from, blockers).contains(to),
            Piece::Rook => rook_moves(from, blockers).contains(to),
            Piece::Queen => {
                (bishop_moves(from, blockers) | rook_moves(from, blockers)).contains(to)
            }
            Piece::King => unreachable!("King moves were already handled"),
        }
    }

    fn is_legal_evasion(&self, mv: Move) -> bool {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        if self.colored_piece_on(from, self.stm) != Some(Piece::King) {
            return false;
        }

        flag == MoveFlag::None
            && king_moves(mv.from())
                .subtract(self.occupied_by(self.stm) | self.attacked)
                .contains(to)
    }

    fn is_legal_en_passant(&self, mv: Move) -> bool {
        self.en_passant.is_some_and(|ep_file| {
            mv.flag() == MoveFlag::EnPassant
                && ep_file.attacker_bb(self.stm).contains(mv.from())
                && mv.to() == Square::new(ep_file.file(), Rank::R6.relative_to(self.stm))
        })
    }

    fn is_legal_castle(&self, mv: Move) -> bool {
        let (from, to) = (mv.from(), mv.to());
        if self.colored_piece_on(from, self.stm) != Some(Piece::King) {
            return false;
        }

        let Some(dir) = [CastlingDirection::Long, CastlingDirection::Short]
            .into_iter()
            .find(|dir| self.castling_rights[self.stm].get(*dir) == Some(to.file()))
        else {
            return false;
        };

        // We now know that `from` contains our king, and that we have a castling right to `to`. The king must therefore
        // be on its starting square, so we don't need to check the ranks of `from` and `to` (since we assume that  any
        // move with the castling flag only moves within one rank). The castling right also implies that `to` contains
        // a rook of ours.

        // Only possible in chess960
        if self.pinned.contains(to) {
            return false;
        }

        let king_dst = Square::new(dir.king_dst(), from.rank());
        let rook_dst = Square::new(dir.rook_dst(), from.rank());

        let must_be_safe = between_inclusive(from, king_dst);
        let must_be_empty = must_be_safe | between_inclusive(from, to) | rook_dst;

        let blockers = self.occupied() ^ from ^ to;

        (must_be_empty & blockers).is_empty() && (must_be_safe & self.attacked).is_empty()
    }
}
