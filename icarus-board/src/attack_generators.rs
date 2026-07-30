// attack generator tables are generated in build.rs
// include!(concat!(env!("OUT_DIR"), "/generated.rs"));

use icarus_common::{bitboard::Bitboard, square::Square, util::Align64};

// static ROOK_RANK_ATTACKS: [[u8; 64]; 8] = {
//     let mut arr = [[0; 64]; 8];
//     let mut rook = 0;
//     while rook < 8 {
//         let mut occ = 0;
//         while occ < 128 {
//             {
//                 let mut file = rook + 1;
//                 while file < 8 {
//                     arr[rook][occ / 2] |= 1 << file;
//                     if (occ & (1 << file)) != 0 {
//                         break;
//                     }
//                     file += 1;
//                 }
//             }

//             {
//                 let mut file = rook.wrapping_sub(1);
//                 while file != usize::MAX {
//                     arr[rook][occ / 2] |= 1 << file;
//                     if (occ & (1 << file)) != 0 {
//                         break;
//                     }
//                     file = file.wrapping_sub(1);
//                 }
//             }

//             occ += 2;
//         }
//         rook += 1;
//     }
//     arr
// };
static ROOK_RANK_ATTACKS: [[u8; 64]; 8] = [
    [
        254, 2, 6, 2, 14, 2, 6, 2, 30, 2, 6, 2, 14, 2, 6, 2, 62, 2, 6, 2, 14, 2, 6, 2, 30, 2, 6, 2,
        14, 2, 6, 2, 126, 2, 6, 2, 14, 2, 6, 2, 30, 2, 6, 2, 14, 2, 6, 2, 62, 2, 6, 2, 14, 2, 6, 2,
        30, 2, 6, 2, 14, 2, 6, 2,
    ],
    [
        253, 253, 5, 5, 13, 13, 5, 5, 29, 29, 5, 5, 13, 13, 5, 5, 61, 61, 5, 5, 13, 13, 5, 5, 29,
        29, 5, 5, 13, 13, 5, 5, 125, 125, 5, 5, 13, 13, 5, 5, 29, 29, 5, 5, 13, 13, 5, 5, 61, 61,
        5, 5, 13, 13, 5, 5, 29, 29, 5, 5, 13, 13, 5, 5,
    ],
    [
        251, 250, 251, 250, 11, 10, 11, 10, 27, 26, 27, 26, 11, 10, 11, 10, 59, 58, 59, 58, 11, 10,
        11, 10, 27, 26, 27, 26, 11, 10, 11, 10, 123, 122, 123, 122, 11, 10, 11, 10, 27, 26, 27, 26,
        11, 10, 11, 10, 59, 58, 59, 58, 11, 10, 11, 10, 27, 26, 27, 26, 11, 10, 11, 10,
    ],
    [
        247, 246, 244, 244, 247, 246, 244, 244, 23, 22, 20, 20, 23, 22, 20, 20, 55, 54, 52, 52, 55,
        54, 52, 52, 23, 22, 20, 20, 23, 22, 20, 20, 119, 118, 116, 116, 119, 118, 116, 116, 23, 22,
        20, 20, 23, 22, 20, 20, 55, 54, 52, 52, 55, 54, 52, 52, 23, 22, 20, 20, 23, 22, 20, 20,
    ],
    [
        239, 238, 236, 236, 232, 232, 232, 232, 239, 238, 236, 236, 232, 232, 232, 232, 47, 46, 44,
        44, 40, 40, 40, 40, 47, 46, 44, 44, 40, 40, 40, 40, 111, 110, 108, 108, 104, 104, 104, 104,
        111, 110, 108, 108, 104, 104, 104, 104, 47, 46, 44, 44, 40, 40, 40, 40, 47, 46, 44, 44, 40,
        40, 40, 40,
    ],
    [
        223, 222, 220, 220, 216, 216, 216, 216, 208, 208, 208, 208, 208, 208, 208, 208, 223, 222,
        220, 220, 216, 216, 216, 216, 208, 208, 208, 208, 208, 208, 208, 208, 95, 94, 92, 92, 88,
        88, 88, 88, 80, 80, 80, 80, 80, 80, 80, 80, 95, 94, 92, 92, 88, 88, 88, 88, 80, 80, 80, 80,
        80, 80, 80, 80,
    ],
    [
        191, 190, 188, 188, 184, 184, 184, 184, 176, 176, 176, 176, 176, 176, 176, 176, 160, 160,
        160, 160, 160, 160, 160, 160, 160, 160, 160, 160, 160, 160, 160, 160, 191, 190, 188, 188,
        184, 184, 184, 184, 176, 176, 176, 176, 176, 176, 176, 176, 160, 160, 160, 160, 160, 160,
        160, 160, 160, 160, 160, 160, 160, 160, 160, 160,
    ],
    [
        127, 126, 124, 124, 120, 120, 120, 120, 112, 112, 112, 112, 112, 112, 112, 112, 96, 96, 96,
        96, 96, 96, 96, 96, 96, 96, 96, 96, 96, 96, 96, 96, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
        64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
    ],
];

#[inline]
pub fn rook_moves(sq: Square, blockers: Bitboard) -> Bitboard {
    rook_bishop_moves(sq, blockers).0
}

#[inline]
pub fn bishop_moves(sq: Square, blockers: Bitboard) -> Bitboard {
    rook_bishop_moves(sq, blockers).1
}

#[inline]
pub fn queen_moves(sq: Square, blockers: Bitboard) -> Bitboard {
    let (orth, diag) = rook_bishop_moves(sq, blockers);
    orth | diag
}

// const fn hq(sq: Square, blockers: Bitboard, mask: Bitboard) -> Bitboard {
//     let r = sq.bitboard().0 << 1;
//     let rr = sq.bitboard().0.swap_bytes() << 1;

//     let o = blockers.0 & mask.0;
//     let fwd = o.wrapping_sub(r);
//     let rev = o.swap_bytes().wrapping_sub(rr);
//     Bitboard((fwd ^ rev.swap_bytes()) & mask.0)
// }

use std::arch::x86_64::*;

static MASKS: Align64<[[Bitboard; 4]; 64]> = {
    let mut arr = [[Bitboard::EMPTY; 4]; 64];
    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        arr[i] = [
            sq.file().bitboard().xor_square(sq),
            Bitboard::main_diag_for(sq).xor_square(sq),
            Bitboard::EMPTY,
            Bitboard::anti_diag_for(sq).xor_square(sq),
        ];
        i += 1;
    }
    Align64(arr)
};

#[inline]
pub fn rook_bishop_moves(sq: Square, blockers: Bitboard) -> (Bitboard, Bitboard) {
    fn bswap(v: __m256i) -> __m256i {
        unsafe {
            _mm256_shuffle_epi8(
                v,
                _mm256_setr_epi8(
                    7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
                    15, 14, 13, 12, 11, 10, 9, 8,
                ),
            )
        }
    }

    unsafe {
        let mask = _mm256_load_si256(MASKS[sq].as_ptr().cast());
        let r = _mm256_set1_epi64x((sq.bitboard().0) as i64);
        let rr = _mm256_set1_epi64x((sq.flip_rank().bitboard().0) as i64);

        let o = _mm256_and_si256(_mm256_set1_epi64x(blockers.0 as i64), mask);
        let fwd = _mm256_sub_epi64(o, r);
        let rev = _mm256_sub_epi64(bswap(o), rr);
        let result = _mm256_and_si256(_mm256_xor_si256(fwd, bswap(rev)), mask);

        let reduced = _mm_or_si128(
            _mm256_castsi256_si128(result),
            _mm256_extracti128_si256(result, 1),
        );

        let rank_shift = 8 * sq.rank().idx();
        let rank_attacks = Bitboard(
            (ROOK_RANK_ATTACKS[sq.file() as usize][(blockers.0 >> (rank_shift + 1)) as usize & 63]
                as u64)
                << rank_shift,
        );

        (
            Bitboard(_mm_extract_epi64(reduced, 0) as u64) | rank_attacks,
            Bitboard(_mm_extract_epi64(reduced, 1) as u64),
        )
    }
}
