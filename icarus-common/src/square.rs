use std::fmt;

use crate::{bitboard::Bitboard, define_enum, piece::Color};

define_enum!(
    #[derive(PartialOrd, Ord, Debug)]
    #[rustfmt::skip]
    pub enum File {
        A, B, C, D, E, F, G, H
    }
);

impl File {
    #[inline]
    pub const fn try_offset(self, offset: i8) -> Option<Self> {
        Self::try_from_idx(self.idx().wrapping_add_signed(offset))
    }

    #[inline]
    pub const fn offset(self, offset: i8) -> Self {
        self.try_offset(offset).expect("Invalid offset")
    }

    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(0x0101010101010101 << self.idx())
    }

    #[inline]
    pub fn to_char(self) -> char {
        (b'A' + self.idx()) as char
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ch = self.to_char();

        if f.alternate() {
            ch = ch.to_ascii_lowercase();
        }

        fmt::Write::write_char(f, ch)
    }
}

define_enum!(
    #[derive(PartialOrd, Ord)]
    #[rustfmt::skip]
    pub enum Rank {
        R1, R2, R3, R4, R5, R6, R7, R8
    }
);

impl Rank {
    #[inline]
    pub const fn try_offset(self, offset: i8) -> Option<Self> {
        Self::try_from_idx(self.idx().wrapping_add_signed(offset))
    }

    #[inline]
    pub const fn offset(self, offset: i8) -> Self {
        self.try_offset(offset).expect("Invalid offset")
    }

    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(0xff << (8 * self.idx()))
    }

    #[inline]
    pub const fn relative_to(self, col: Color) -> Self {
        Self::from_idx(self.idx() ^ (7 * col.idx()))
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&(self.idx() + 1), f)
    }
}

impl fmt::Debug for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&(self.idx() + 1), f)
    }
}

define_enum!(
    #[derive(Debug)]
    #[rustfmt::skip]
    pub enum Square {
        A1, B1, C1, D1, E1, F1, G1, H1,
        A2, B2, C2, D2, E2, F2, G2, H2,
        A3, B3, C3, D3, E3, F3, G3, H3,
        A4, B4, C4, D4, E4, F4, G4, H4,
        A5, B5, C5, D5, E5, F5, G5, H5,
        A6, B6, C6, D6, E6, F6, G6, H6,
        A7, B7, C7, D7, E7, F7, G7, H7,
        A8, B8, C8, D8, E8, F8, G8, H8,
    }
);

impl Square {
    #[inline]
    pub const fn new(file: File, rank: Rank) -> Self {
        Self::from_idx(file.idx() + rank.idx() * 8)
    }

    #[inline]
    pub const fn file(self) -> File {
        File::from_idx(self.idx() & 7)
    }

    #[inline]
    pub const fn rank(self) -> Rank {
        Rank::from_idx(self.idx() >> 3)
    }

    #[inline]
    pub const fn try_offset(self, df: i8, dr: i8) -> Option<Self> {
        match (self.file().try_offset(df), self.rank().try_offset(dr)) {
            (Some(f), Some(r)) => Some(Self::new(f, r)),
            _ => None,
        }
    }

    #[inline]
    pub const fn offset(self, df: i8, dr: i8) -> Self {
        self.try_offset(df, dr).expect("Invalid offset")
    }

    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(1 << self.idx())
    }

    #[inline]
    pub fn parse(s: &str) -> Option<Self> {
        let &[f, r]: &[u8; 2] = s.as_bytes().try_into().ok()?;

        match (f.to_ascii_lowercase(), r) {
            (f @ b'a'..=b'h', b'1'..=b'8') => Some(Self::new(
                File::from_idx(f - b'a'),
                Rank::from_idx(r - b'1'),
            )),
            _ => None,
        }
    }

    #[inline]
    pub fn flip_rank(self) -> Self {
        Self::from_idx(self.idx() ^ 56)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.file(), f)?;
        fmt::Display::fmt(&self.rank(), f)
    }
}
