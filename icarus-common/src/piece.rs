use std::{fmt, ops::Not};

use crate::define_enum;

define_enum!(
    pub enum Piece {
        Pawn,
        Knight,
        Bishop,
        Rook,
        Queen,
        King,
    }
);

impl Piece {
    #[inline]
    pub fn to_char(self, color: Color) -> char {
        let mut ch = match self {
            Piece::Knight => 'N',
            Piece::Bishop => 'B',
            Piece::Rook => 'R',
            Piece::Queen => 'Q',
            Piece::Pawn => 'P',
            Piece::King => 'K',
        };

        if color == Color::Black {
            ch = ch.to_ascii_lowercase();
        }

        ch
    }

    #[inline]
    pub fn from_char(ch: char) -> Option<Self> {
        match ch.to_ascii_lowercase() {
            'n' => Some(Piece::Knight),
            'b' => Some(Piece::Bishop),
            'r' => Some(Piece::Rook),
            'q' => Some(Piece::Queen),
            'p' => Some(Piece::Pawn),
            'k' => Some(Piece::King),
            _ => None,
        }
    }
}

define_enum!(
    #[derive(Debug)]
    pub enum Color {
        White,
        Black,
    }
);

impl Color {
    #[inline]
    pub const fn invert(self) -> Self {
        Self::from_idx(1 - self.idx())
    }

    #[inline]
    /// Returns 1 for white, -1 for black
    pub const fn signum(self) -> i8 {
        1 - 2 * self.idx() as i8
    }

    #[inline]
    pub const fn to_char(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }
}

impl Not for Color {
    type Output = Self;

    #[inline]
    fn not(self) -> Self {
        self.invert()
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Write::write_char(f, self.to_char())
    }
}
