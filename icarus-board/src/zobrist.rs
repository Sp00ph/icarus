use icarus_common::{
    piece::{Color, Piece},
    square::{File, Square},
};

struct Xoshiro256PlusPlus([u64; 4]);

impl Xoshiro256PlusPlus {
    const fn next(&mut self) -> u64 {
        let s = &mut self.0;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];

        s[2] ^= t;
        s[3] = s[3].rotate_left(45);

        result
    }
}

pub struct Zobrist {
    pieces: [[[u64; Square::COUNT]; Piece::COUNT]; Color::COUNT],
    pub black_to_move: u64,
    castles: [[u64; File::COUNT]; Color::COUNT],
    en_passant: [u64; File::COUNT],
}

impl Zobrist {
    #[inline]
    pub const fn piece(&self, sq: Square, pt: Piece, col: Color) -> u64 {
        self.pieces[col as usize][pt as usize][sq as usize]
    }

    #[inline]
    pub const fn castles(&self, file: File, col: Color) -> u64 {
        self.castles[col as usize][file as usize]
    }

    #[inline]
    pub const fn en_passant(&self, file: File) -> u64 {
        self.en_passant[file as usize]
    }
}

pub static ZOBRIST: Zobrist = {
    let mut zobrist = Zobrist {
        pieces: [[[0; Square::COUNT]; Piece::COUNT]; Color::COUNT],
        black_to_move: 0,
        castles: [[0; File::COUNT]; Color::COUNT],
        en_passant: [0; File::COUNT],
    };

    // Random 256-bit key
    let mut rng = Xoshiro256PlusPlus([
        0x9b388c2766f7a6c3,
        0xa7ea07dd648dc636,
        0x8b7eb148fafc6178,
        0xb5099720c02129c1,
    ]);

    let mut c = 0;
    while c < Color::COUNT {
        let mut p = 0;
        while p < Piece::COUNT {
            let mut s = 0;
            while s < Square::COUNT {
                zobrist.pieces[c][p][s] = rng.next();
                s += 1;
            }
            p += 1;
        }

        let mut f = 0;
        while f < File::COUNT {
            zobrist.castles[c][f] = rng.next();
            zobrist.en_passant[f] = rng.next();
            f += 1;
        }
        c += 1;
    }

    zobrist.black_to_move = rng.next();

    zobrist
};
