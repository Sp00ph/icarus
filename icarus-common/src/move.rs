use std::{fmt, num::NonZeroU16};

use crate::{
    bitboard::Bitboard,
    define_enum,
    piece::{Color, Piece},
    square::{File, Square},
};

/// Bit packed move type
/// Bits 0-5:   src square
///      6-11:  dst square
///      12-13: move flag
///      14-15: promotion piece type (N, B, R, Q)
///
/// Castles is encoded as king captures rook.
/// Because src and dst square are always distinct, at most one
/// of them may be zero, so a valid move will always be nonzero.
/// We use NonZeroU16 to enable 0 as a niche value.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(NonZeroU16);

define_enum!(
    #[derive(Debug)]
    pub enum MoveFlag {
        None,
        Castle,
        EnPassant,
        Promotion,
    }
);

impl Move {
    #[inline]
    pub const fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        debug_assert!(!matches!(flag, MoveFlag::Promotion));

        Self(
            NonZeroU16::new(
                (from.idx() as u16) | ((to.idx() as u16) << 6) | ((flag.idx() as u16) << 12),
            )
            .expect("Invalid move squares"),
        )
    }

    #[inline(never)]
    pub const fn new_promotion(from: Square, to: Square, promote_to: Piece) -> Self {
        debug_assert!(
            promote_to.idx() != Piece::Pawn.idx() && promote_to.idx() != Piece::King.idx()
        );

        Self(
            NonZeroU16::new(
                (from.idx() as u16)
                    | ((to.idx() as u16) << 6)
                    | ((MoveFlag::Promotion.idx() as u16) << 12)
                    | (((promote_to.idx() - Piece::Knight.idx()) as u16) << 14),
            )
            .unwrap(),
        )
    }

    #[inline]
    pub const fn from(self) -> Square {
        Square::from_idx(self.0.get() as u8 & 0x3f)
    }

    #[inline]
    pub const fn to(self) -> Square {
        Square::from_idx((self.0.get() >> 6) as u8 & 0x3f)
    }

    #[inline]
    pub const fn flag(self) -> MoveFlag {
        MoveFlag::from_idx((self.0.get() >> 12) as u8 & 0x3)
    }

    #[inline]
    pub const fn promotes_to(self) -> Option<Piece> {
        if !matches!(self.flag(), MoveFlag::Promotion) {
            None
        } else {
            Some(self.promotes_to_unchecked())
        }
    }

    /// Returns the piece that this move promotes to. If the move
    /// doesn't promote, the return value is arbitrary.
    #[inline]
    pub const fn promotes_to_unchecked(self) -> Piece {
        Piece::from_idx(((self.0.get() >> 14) as u8 & 0x3) + Piece::Knight.idx())
    }

    #[inline]
    pub const fn to_bits(self) -> u16 {
        self.0.get()
    }

    /// Should only be called with an argument that was previously returned from a `mov.to_bits()` call.
    #[inline]
    pub const fn from_bits(n: u16) -> Self {
        Self(NonZeroU16::new(n).expect("Illegal move!"))
    }

    pub fn display(self, chess960: bool) -> String {
        let from = self.from();
        let mut to = self.to();
        if !chess960 && self.flag() == MoveFlag::Castle {
            let to_file = if to.file() < from.file() {
                File::C
            } else {
                File::G
            };
            to = Square::new(to_file, from.rank());
        }

        let mut s = format!("{:#}{:#}", from, to);
        if let Some(p) = self.promotes_to() {
            s.push(p.to_char(Color::Black));
        }
        s
    }
}

impl fmt::Debug for Move {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.display(true), f)
    }
}

#[derive(Clone)]
pub struct PieceMoves {
    move_flag: MoveFlag,
    piece: Piece,
    from: Square,
    to: Bitboard,
}

impl PieceMoves {
    #[inline]
    pub const fn new(move_flag: MoveFlag, piece: Piece, from: Square, to: Bitboard) -> Self {
        Self {
            move_flag,
            piece,
            from,
            to,
        }
    }

    #[inline]
    pub const fn piece_type(&self) -> Piece {
        self.piece
    }

    #[inline]
    pub const fn to(&self) -> Bitboard {
        self.to
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.to.popcnt() as usize
            * if self.move_flag.idx() == MoveFlag::Promotion.idx() {
                4
            } else {
                1
            }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.to.is_empty()
    }
}

#[derive(Clone)]
pub struct PieceMovesIter {
    moves: PieceMoves,
    promote_idx: u8,
}

impl Iterator for PieceMovesIter {
    type Item = Move;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let from = self.moves.from;
        let to = self.moves.to.try_next()?;

        if self.moves.move_flag == MoveFlag::Promotion {
            let mov = Move::new_promotion(
                from,
                to,
                // Promote to queen first
                Piece::from_idx(4 - self.promote_idx),
            );
            self.promote_idx += 1;
            if self.promote_idx >= 4 {
                self.promote_idx = 0;
                self.moves.to ^= to;
            }

            Some(mov)
        } else {
            self.moves.to ^= to;
            Some(Move::new(from, to, self.moves.move_flag))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.moves.len() - self.promote_idx as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for PieceMovesIter {}

impl IntoIterator for PieceMoves {
    type Item = Move;

    type IntoIter = PieceMovesIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        PieceMovesIter {
            moves: self,
            promote_idx: 0,
        }
    }
}
