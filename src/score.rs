use std::{
    fmt,
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::util::MAX_PLY;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(pub i16);

impl Score {
    pub fn new(score: i16) -> Self {
        Self(score)
    }

    pub fn new_mate(ply: u16) -> Self {
        Self::MIN_MATE - ply as i16
    }

    pub fn new_mated(ply: u16) -> Self {
        -Self::new_mate(ply)
    }

    pub fn is_mate(self) -> bool {
        (Self::MAX_MATE.0..=Self::MIN_MATE.0).contains(&self.0.abs())
    }

    pub fn is_infinite(self) -> bool {
        self.0.abs() >= Self::INFINITE.0
    }

    pub fn mate_in(self) -> Option<i16> {
        if self.is_mate() {
            let abs_score = self.0.abs();
            let sign = self.0.signum();

            return Some(sign * (Score::MIN_MATE.0 - abs_score));
        }

        None
    }

    pub fn clamp_nomate(score: i16) -> Self {
        Score(score).clamp(-Score::MAX_MATE + 1, Score::MAX_MATE - 1)
    }

    /// Corresponds to "Mate in 0"
    pub const MIN_MATE: Self = Self(i16::MAX - MAX_PLY as i16);
    /// Corresponds to Mate in MAX_PLY.
    pub const MAX_MATE: Self = Self(i16::MAX - (2 * MAX_PLY) as i16);
    pub const ZERO: Self = Self(0);
    pub const INFINITE: Self = Self(Self::MIN_MATE.0 + 1);
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            if self.is_infinite() {
                if self.0 > 0 {
                    write!(f, "+INF")
                } else {
                    write!(f, "-INF")
                }
            } else if let Some(ply) = self.mate_in() {
                write!(f, "#{}", (ply + ply.signum()) / 2)
            } else {
                write!(f, "{:+.2}", self.0 as f32 / 100.0)
            }
        } else if let Some(ply) = self.mate_in() {
            write!(f, "mate {}", (ply + ply.signum()) / 2)
        } else {
            write!(f, "cp {}", self.0)
        }
    }
}

impl Neg for Score {
    type Output = Score;

    #[inline]
    fn neg(self) -> Self::Output {
        Score(-self.0)
    }
}

macro_rules! impl_binops {
    ($(($trait:ident, $fn:ident))+) => {
        $(
            impl $trait for Score {
                type Output = Self;

                fn $fn(self, rhs: Self) -> Self {
                    Self($trait::$fn(self.0, rhs.0))
                }
            }
        )+
    };
}

macro_rules! impl_scalar_binops {
    ($(($trait:ident, $fn:ident))+) => {
        $(
            impl $trait<i16> for Score {
                type Output = Self;

                fn $fn(self, rhs: i16) -> Self {
                    Self($trait::$fn(self.0, rhs))
                }
            }
        )+
    };
}

impl_binops!((Add, add)(Sub, sub));

impl_scalar_binops!((Add, add)(Sub, sub)(Mul, mul)(Div, div));
