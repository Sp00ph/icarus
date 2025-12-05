use std::arch::x86_64::*;

use icarus_common::{
    bitboard::Bitboard,
    lookups::{bishop_rays, rook_rays},
    square::Square,
};

fn expand(x: u32) -> __m256i {
    unsafe {
        let v = _mm256_set1_epi32(x as i32);
        let shuf = _mm256_setr_epi64x(
            0x0000000000000000,
            0x0101010101010101,
            0x0202020202020202,
            0x0303030303030303,
        );
        let mask = _mm256_set1_epi64x(0x8040201008040201u64 as i64);

        _mm256_cmpeq_epi8(_mm256_and_si256(_mm256_shuffle_epi8(v, shuf), mask), mask)
    }
}

fn perm8(v: __m256i, idx: __m256i) -> __m256i {
    unsafe {
        let mask = _mm256_slli_epi16(idx, 3);
        let vlo = _mm256_permute2x128_si256(v, v, 0x00);
        let vhi = _mm256_permute2x128_si256(v, v, 0x11);

        _mm256_blendv_epi8(
            _mm256_shuffle_epi8(vlo, idx),
            _mm256_shuffle_epi8(vhi, idx),
            mask,
        )
    }
}

// SAFETY: `perm` must be 32-byte aligned.
fn bitshuffle(x: u32, perm: &[u8; 32]) -> u32 {
    unsafe {
        _mm256_movemask_epi8(perm8(expand(x), _mm256_load_si256(perm.as_ptr().cast()))) as u32
    }
}

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub(super) struct SquareInfo {
    pub perm: [u8; 32],
    pub iperm: [u8; 32],
    pub pext: u64,
    pub pdep: u64,
}

const fn calc_queen_perm_and_iperm(sq: Square, pext: u64) -> ([u8; 32], [u8; 32]) {
    let mut perm = [0; 32];
    let mut iperm = [0; 32];

    let dirs = [
        (0, -1),
        (-1, -1),
        (-1, 0),
        (1, -1),
        (0, 1),
        (1, 1),
        (1, 0),
        (-1, 1),
    ];

    let mut i = 0;
    let mut dst = 0;
    while i < dirs.len() {
        let (df, dr) = dirs[i];
        let mut cur = sq;
        while let Some(sq) = cur.try_offset(df, dr) {
            cur = sq;
            let lower_mask = pext & !(u64::MAX << cur.idx());
            let src = lower_mask.count_ones();
            perm[dst] = src as u8;
            iperm[src as usize] = dst as u8;
            dst += 1;
        }

        i += 1;
    }

    (perm, iperm)
}

pub(super) static QUEEN_INFOS: [SquareInfo; 64] = {
    let mut arr = [SquareInfo {
        perm: [0; 32],
        iperm: [0; 32],
        pext: 0,
        pdep: 0,
    }; 64];

    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        let (file, rank) = (sq.file().idx() as u32, sq.rank().idx() as u32);
        arr[i].pext = rook_rays(sq).0 | bishop_rays(sq).0;

        let n_south = rank;
        let n_sw = if rank < file { rank } else { file };
        let n_west = file;
        let n_se = if rank < 7 - file { rank } else { 7 - file };
        let n_north = 7 - rank;
        let n_ne = if file > rank { 7 - file } else { 7 - rank };
        let n_east = 7 - file;
        let n_nw = if file < 7 - rank { file } else { 7 - rank };

        let ray_lens = [n_south, n_sw, n_west, n_se, n_north, n_ne, n_east, n_nw];

        let mut j = 0;
        while j < ray_lens.len() {
            arr[i].pdep |= ((1u64 << ray_lens[j]) - 1) << (8 * j);
            j += 1;
        }

        (arr[i].perm, arr[i].iperm) = calc_queen_perm_and_iperm(sq, arr[i].pext);

        i += 1;
    }

    arr
};

pub fn queen_to_ray_space(sq: Square, bb: Bitboard) -> Bitboard {
    let SquareInfo {
        ref perm,
        pext,
        pdep,
        ..
    } = QUEEN_INFOS[sq.idx() as usize];
    unsafe {
        let compressed = _pext_u64(bb.0, pext) as u32;
        let shuffled = bitshuffle(compressed, perm);
        Bitboard(_pdep_u64(shuffled as u64, pdep))
    }
}

pub fn queen_from_ray_space(sq: Square, rays: Bitboard) -> Bitboard {
    let SquareInfo {
        ref iperm,
        pext,
        pdep,
        ..
    } = QUEEN_INFOS[sq.idx() as usize];
    unsafe {
        let compressed = _pext_u64(rays.0, pdep) as u32;
        let shuffled = bitshuffle(compressed, iperm);
        Bitboard(_pdep_u64(shuffled as u64, pext))
    }
}
