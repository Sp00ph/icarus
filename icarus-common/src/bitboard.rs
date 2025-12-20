use crate::{
    direction::{Direction, Up, shift_bb},
    square::{File, Rank, Square},
};
use std::{
    iter::FusedIterator,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Self = Self(0);

    pub const ALL: Self = Self(u64::MAX);

    /// A1-H8 diagonal
    pub const MAIN_DIAGONAL: Self = Self(0x8040201008040201);

    /// A8-H1 diagonal
    pub const ANTI_DIAGONAL: Self = Self(0x0102040810204080);

    pub const LIGHT_SQUARES: Self = Self(0xAA55AA55AA55AA55);
    
    pub const DARK_SQUARES: Self = Self(0x55AA55AA55AA55AA);

    #[inline]
    pub const fn invert(self) -> Self {
        Self(!self.0)
    }

    #[inline]
    pub const fn subtract(self, rhs: Self) -> Self {
        self.intersect(rhs.invert())
    }

    #[inline]
    pub const fn xor_square(self, sq: Square) -> Self {
        self.xor(sq.bitboard())
    }

    #[inline]
    pub const fn contains(self, sq: Square) -> bool {
        self.intersect(sq.bitboard()).is_non_empty()
    }

    #[inline]
    pub const fn is_subset_of(self, superset: Self) -> bool {
        self.intersect(superset).0 == self.0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_non_empty(self) -> bool {
        !self.is_empty()
    }

    #[inline]
    pub const fn try_next(self) -> Option<Square> {
        Square::try_from_idx(self.0.trailing_zeros() as u8)
    }

    #[inline]
    pub const fn next(self) -> Square {
        self.try_next().expect("Called next() on an empty bitboard")
    }

    #[inline]
    pub const fn popcnt(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[inline]
    pub const fn shift_const<D: Direction, const STEPS: u8>(self) -> Self {
        shift_bb::<D>(self, STEPS as i8)
    }

    #[inline]
    pub const fn shift<D: Direction>(self, steps: i8) -> Self {
        shift_bb::<D>(self, steps)
    }

    #[inline]
    pub const fn main_diag_for(sq: Square) -> Self {
        let shift = sq.rank().idx() as i8 - sq.file().idx() as i8;
        Self::MAIN_DIAGONAL.shift::<Up>(shift)
    }

    #[inline]
    pub const fn anti_diag_for(sq: Square) -> Self {
        let shift = sq.rank().idx() as i8 + sq.file().idx() as i8 - 7;
        Self::ANTI_DIAGONAL.shift::<Up>(shift)
    }
}

macro_rules! binop {
    ($(($trait:ident, $tfn:ident, $assign_trait:ident, $atfn:ident, $fn: ident, $op:tt),)+) => {
        impl Bitboard {
            $(
                #[inline]
                pub const fn $fn(self, rhs: Self) -> Self {
                    Self(self.0 $op rhs.0)
                }
            )+
        }

        $(
            impl $trait for Bitboard {
                type Output = Self;

                #[inline]
                fn $tfn(self, rhs: Self) -> Self {
                    self.$fn(rhs)
                }
            }

            impl $trait<Square> for Bitboard {
                type Output = Self;

                #[inline]
                fn $tfn(self, rhs: Square) -> Self {
                    self.$fn(rhs.bitboard())
                }
            }

            impl $trait<File> for Bitboard {
                type Output = Self;

                #[inline]
                fn $tfn(self, rhs: File) -> Self {
                    self.$fn(rhs.bitboard())
                }
            }

            impl $trait<Rank> for Bitboard {
                type Output = Self;

                #[inline]
                fn $tfn(self, rhs: Rank) -> Self {
                    self.$fn(rhs.bitboard())
                }
            }

            impl $assign_trait for Bitboard {
                #[inline]
                fn $atfn(&mut self, rhs: Self) {
                    *self = self.$fn(rhs)
                }
            }

            impl $assign_trait<Square> for Bitboard {
                #[inline]
                fn $atfn(&mut self, rhs: Square) {
                    *self = self.$fn(rhs.bitboard())
                }
            }

            impl $assign_trait<File> for Bitboard {
                #[inline]
                fn $atfn(&mut self, rhs: File) {
                    *self = self.$fn(rhs.bitboard())
                }
            }

            impl $assign_trait<Rank> for Bitboard {
                #[inline]
                fn $atfn(&mut self, rhs: Rank) {
                    *self = self.$fn(rhs.bitboard())
                }
            }
        )+
    };
}

binop!(
    (BitAnd, bitand, BitAndAssign, bitand_assign, intersect, &),
    (BitOr, bitor, BitOrAssign, bitor_assign, union, |),
    (BitXor, bitxor, BitXorAssign, bitxor_assign, xor, ^),
);

impl Not for Bitboard {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        self.invert()
    }
}

#[derive(Clone)]
pub struct BitboardIter(Bitboard);

impl Iterator for BitboardIter {
    type Item = Square;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sq = self.0.try_next()?;
        self.0 = self.0.xor_square(sq);
        Some(sq)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();

        (n, Some(n))
    }
}

impl ExactSizeIterator for BitboardIter {
    #[inline]
    fn len(&self) -> usize {
        self.0.popcnt() as _
    }
}

impl FusedIterator for BitboardIter {}

impl IntoIterator for Bitboard {
    type Item = Square;

    type IntoIter = BitboardIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        BitboardIter(self)
    }
}

impl FromIterator<Square> for Bitboard {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Square>>(iter: T) -> Self {
        let mut bb = Self::EMPTY;
        bb.extend(iter);
        bb
    }
}

impl Extend<Square> for Bitboard {
    #[inline]
    fn extend<T: IntoIterator<Item = Square>>(&mut self, iter: T) {
        *self = iter.into_iter().fold(*self, |bb, sq| bb | sq)
    }
}
