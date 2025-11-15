use std::num::NonZeroU8;

use icarus_common::{
    bitboard::Bitboard,
    piece::Color,
    square::{File, Rank, Square},
};

/// Bitpacked type containing information about which file we can take
/// en passant on, and which pawns can take. After making a move, we
/// only set the en passant file if it can legally be taken. By storing
/// the attacker information alongside it, we can skip a redundant legality
/// check when generating the next moves.
///
/// Bits 0-2: file
///        3: left may take
///        4: right may take
///
/// Note that at least one of bits 3 and 4 will always be set.
#[derive(Clone, Copy)]
pub struct EnPassantFile(NonZeroU8);

impl EnPassantFile {
    #[inline]
    pub fn new(file: File, left: bool, right: bool) -> Self {
        Self(
            NonZeroU8::new(file.idx() | (u8::from(left) << 3) | (u8::from(right) << 4))
                .expect("Invalid EP file"),
        )
    }

    #[inline]
    pub fn file(self) -> File {
        File::from_idx(self.0.get() & 7)
    }

    #[inline]
    pub fn attacker_bb(self, stm: Color) -> Bitboard {
        let rank = Rank::R5.relative_to(stm);
        let ep_sq = Square::new(self.file(), rank);
        let mut bb = Bitboard::EMPTY;

        if (self.0.get() >> 3) & 1 != 0 {
            bb |= Square::from_idx(ep_sq.idx() - 1);
        }
        if (self.0.get() >> 4) & 1 != 0 {
            bb |= Square::from_idx(ep_sq.idx() + 1);
        }

        bb
    }
}
