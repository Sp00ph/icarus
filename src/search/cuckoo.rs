use std::mem;

use crate::{
    board::zobrist::ZOBRIST,
    common::{
        bitboard::Bitboard,
        lookups::{bishop_rays, king_moves, knight_moves, rook_rays},
        piece::{Color, Piece},
        square::Square,
    },
    tagged_cell,
};

pub const fn h1(key: u64) -> usize {
    (key & 0x1FFF) as usize
}

pub const fn h2(key: u64) -> usize {
    ((key >> 32) & 0x1FFF) as usize
}

fn attacks_by_type(piece: Piece, sq: Square) -> Bitboard {
    match piece {
        Piece::Pawn => panic!("Didn't expect pawns in cuckoo initialization"),
        Piece::Knight => knight_moves(sq),
        Piece::Bishop => bishop_rays(sq),
        Piece::Rook => rook_rays(sq),
        Piece::Queen => bishop_rays(sq) | rook_rays(sq),
        Piece::King => king_moves(sq),
    }
}

#[derive(zerocopy::FromZeros)]
pub struct Cuckoo {
    pub keys: [u64; 0x2000],
    pub moves: [(Square, Square); 0x2000],
}

impl Cuckoo {
    pub fn init() -> CuckooTag {
        CUCKOO.init_inplace(|cuckoo| {
            for piece in Piece::all().filter(|piece| *piece != Piece::Pawn) {
                for color in Color::all() {
                    for src in Square::all() {
                        for dst in Square::all()
                            .filter(|dst| dst.idx() > src.idx())
                            .filter(|dst| attacks_by_type(piece, src).contains(*dst))
                        {
                            let mut mv = (src, dst);
                            let mut key = ZOBRIST.piece(src, piece, color)
                                ^ ZOBRIST.piece(dst, piece, color)
                                ^ ZOBRIST.black_to_move;
                            let mut slot = h1(key);
                            while mv != (Square::A1, Square::A1) {
                                mem::swap(&mut cuckoo.keys[slot], &mut key);
                                mem::swap(&mut cuckoo.moves[slot], &mut mv);

                                slot = if slot == h1(key) { h2(key) } else { h1(key) };
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn get(tag: CuckooTag) -> &'static Self {
        CUCKOO.get(tag)
    }
}

tagged_cell!(
    static CUCKOO: TaggedCell<Cuckoo, pub CuckooTag> = TaggedCell::new();
);
