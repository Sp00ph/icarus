use std::arch::x86_64::*;

use icarus_common::{
    bitboard::Bitboard,
    square::{File, Rank},
};

const A: i64 = File::A.bitboard().0 as i64;
const B: i64 = File::B.bitboard().0 as i64;
const G: i64 = File::G.bitboard().0 as i64;
const H: i64 = File::H.bitboard().0 as i64;
const R1: i64 = Rank::R1.bitboard().0 as i64;
const R2: i64 = Rank::R2.bitboard().0 as i64;
const R7: i64 = Rank::R7.bitboard().0 as i64;
const R8: i64 = Rank::R8.bitboard().0 as i64;

fn knight_attacks_setwise(knights: Bitboard) -> __m512i {
    unsafe {
        // knight moves are done clockwise, starting at wnw.
        let rotates = _mm512_setr_epi64(6, 15, 17, 10, -6, -15, -17, -10);
        // mask containing the files+ranks that need to be removed for each shift
        // (e.g. a knight that is on files a or b or on rank 8 cannot move wnw).
        let mask = _mm512_setr_epi64(
            A | B | R8,
            A | R7 | R8,
            H | R7 | R8,
            G | H | R8,
            G | H | R1,
            H | R1 | R2,
            A | R1 | R2,
            A | B | R1,
        );

        _mm512_rolv_epi64(
            _mm512_andnot_si512(mask, _mm512_set1_epi64(knights.0 as i64)),
            rotates,
        )
    }
}

fn slider_attacks_setwise(orth: Bitboard, diag: Bitboard, blockers: Bitboard) -> __m512i {
    unsafe {
        let (orth, diag) = (orth.0 as i64, diag.0 as i64);
        let rotate = |n: i64| _mm512_setr_epi64(-7 * n, -9 * n, 7 * n, 9 * n, n, -8 * n, -n, 8 * n);
        // se, sw, nw, ne, e, s, w, n
        let mut generate = _mm512_setr_epi64(diag, diag, diag, diag, orth, orth, orth, orth);
        let mask = _mm512_setr_epi64(A | R8, H | R8, H | R1, A | R1, A, R8, H, R1);
        let mut blockers = _mm512_or_si512(mask, _mm512_set1_epi64(blockers.0 as i64));

        // 242 <=> a | (!b & c)
        generate = _mm512_ternarylogic_epi64(
            generate,
            blockers,
            _mm512_rolv_epi64(generate, rotate(1)),
            242,
        );
        blockers = _mm512_or_si512(blockers, _mm512_rolv_epi64(blockers, rotate(1)));

        generate = _mm512_ternarylogic_epi64(
            generate,
            blockers,
            _mm512_rolv_epi64(generate, rotate(2)),
            242,
        );
        blockers = _mm512_or_si512(blockers, _mm512_rolv_epi64(blockers, rotate(2)));

        generate = _mm512_ternarylogic_epi64(
            generate,
            blockers,
            _mm512_rolv_epi64(generate, rotate(4)),
            242,
        );

        _mm512_andnot_si512(mask, _mm512_rolv_epi64(generate, rotate(1)))
    }
}

pub fn knight_and_slider_attacks_setwise(
    knights: Bitboard,
    orth: Bitboard,
    diag: Bitboard,
    blockers: Bitboard,
) -> Bitboard {
    unsafe {
        Bitboard(_mm512_reduce_or_epi64(_mm512_or_si512(
            knight_attacks_setwise(knights),
            slider_attacks_setwise(orth, diag, blockers),
        )) as u64)
    }
}
