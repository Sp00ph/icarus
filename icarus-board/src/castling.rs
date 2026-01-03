use icarus_common::square::File;

/// The castling rights for one side. We need to store flags for whether
/// the side may castle short or long. Additionally, to support Chess960,
/// we need to store the initial files of the rooks. We pack the bits as follows:
///
/// Bits 0-2: long file
/// Bit  3:   long flag
/// Bits 4-6: short file
/// Bit  7:   short flag
///
/// For the flags, we use 0 to signal that the king may castle in that direction.
/// This way, a nibble represents a valid file to castle to iff it contains
/// a value in 0..8, which is just the file index to castle to.
#[derive(Clone, Copy)]
pub struct CastlingRights(u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastlingDirection {
    Long = 0,
    Short = 4,
}

impl CastlingDirection {
    #[inline]
    pub fn king_dst(self) -> File {
        match self {
            Self::Long => File::C,
            Self::Short => File::G,
        }
    }

    #[inline]
    pub fn rook_dst(self) -> File {
        match self {
            Self::Long => File::D,
            Self::Short => File::F,
        }
    }
}

impl CastlingRights {
    #[inline]
    pub fn new(long: Option<File>, short: Option<File>) -> Self {
        let long = long.map_or(8, File::idx);
        let short = short.map_or(8, File::idx);
        Self(long | (short << 4))
    }

    #[inline]
    pub fn get(self, dir: CastlingDirection) -> Option<File> {
        File::try_from_idx((self.0 >> dir as u8) & 0xf)
    }

    #[inline]
    pub fn set(&mut self, dir: CastlingDirection, file: Option<File>) {
        let and_mask = !(0xf << (dir as u8));
        let new_val = file.map_or(8, File::idx);
        self.0 = (self.0 & and_mask) | (new_val << (dir as u8));
    }
}

impl Default for CastlingRights {
    fn default() -> Self {
        Self::new(None, None)
    }
}
