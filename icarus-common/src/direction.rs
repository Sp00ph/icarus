use crate::bitboard::Bitboard;

pub trait Direction {
    /// File delta
    const DF: i8;
    /// Rank delta
    const DR: i8;

    type Opposite: Direction;
}

pub struct Up;
pub struct UpRight;
pub struct Right;
pub struct DownRight;
pub struct Down;
pub struct DownLeft;
pub struct Left;
pub struct UpLeft;

macro_rules! impl_dir {
    ($(($dir:ident, $opp:ident, $df:literal, $dr:literal),)*) => {
        $(
            impl Direction for $dir {
                type Opposite = $opp;
                const DF: i8 = $df;
                const DR: i8 = $dr;
            }
        )*
    };
}

impl_dir!(
    (Up, Down, 0, 1),
    (Right, Left, 1, 0),
    (Down, Up, 0, -1),
    (Left, Right, -1, 0),
    (UpRight, DownLeft, 1, 1),
    (DownRight, UpLeft, 1, -1),
    (DownLeft, UpRight, -1, -1),
    (UpLeft, DownRight, -1, 1),
);

#[inline]
pub(crate) const fn shl_general(bb: Bitboard, shift: i8) -> Bitboard {
    Bitboard(if shift >= 0 {
        bb.0 << shift
    } else {
        bb.0 >> (-shift)
    })
}

#[inline]
pub const fn horizontal_shl_mask(shift: i8) -> Bitboard {
    Bitboard(u64::from_ne_bytes(
        [if shift >= 0 {
            0xff << shift
        } else {
            0xff >> -shift
        }; 8],
    ))
}

#[inline]
pub(crate) const fn shift_bb<D: Direction>(bb: Bitboard, steps: i8) -> Bitboard {
    let mask = horizontal_shl_mask(D::DF * steps);
    let shift_amt = (8 * D::DR + D::DF) * steps;
    shl_general(bb, shift_amt).intersect(mask)
}
