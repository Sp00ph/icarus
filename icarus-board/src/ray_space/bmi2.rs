use icarus_common::{bitboard::Bitboard, square::Square};

/// pext/pdep mask pairs to convert from bitboard space to ray space.
/// For square sq, PEXT_PDEP[sq] contains (in order) the pairs to
/// convert file, rank, diagonal and antidiagonal to ray space.
static PEXT_PDEP: [[[u64; 2]; 4]; 64] = {
    let mut arr = [[[0u64; 2]; 4]; 64];
    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        let (file, rank) = (sq.file().idx() as u32, sq.rank().idx() as u32);

        let n_south = rank;
        let n_north = 7 - rank;
        let n_west = file;
        let n_east = 7 - file;
        let n_se = if rank < 7 - file { rank } else { 7 - file };
        let n_nw = if file < 7 - rank { file } else { 7 - rank };
        let n_sw = if rank < file { rank } else { file };
        let n_ne = if file > rank { 7 - file } else { 7 - rank };

        arr[i][0][0] = sq.file().bitboard().subtract(sq.bitboard()).0;
        arr[i][0][1] = (((1u64 << n_south) - 1) << (8 - n_south)) | (((1u64 << n_north) - 1) << 32);

        arr[i][1][0] = sq.rank().bitboard().subtract(sq.bitboard()).0;
        arr[i][1][1] = (((1u64 << n_west) - 1) << (24 - n_west)) | (((1u64 << n_east) - 1) << 48);

        arr[i][2][0] = Bitboard::main_diag_for(sq).subtract(sq.bitboard()).0;
        arr[i][2][1] = (((1u64 << n_se) - 1) << (32 - n_se)) | (((1u64 << n_nw) - 1) << 56);

        arr[i][3][0] = Bitboard::anti_diag_for(sq).subtract(sq.bitboard()).0;
        arr[i][3][1] = (((1u64 << n_sw) - 1) << (16 - n_sw)) | (((1u64 << n_ne) - 1) << 40);

        i += 1;
    }
    arr
};

/// Convert a bitboard into ray space format.
/// Each byte corresponds to one ray, in one of the 8 compass directions.
/// From least to most significant byte, they are ordered S, SW, W, SE, N, NE, E, NW.
/// Within each byte, the least significant bit corresponds to the square closest to
/// the source.
#[inline]
pub fn queen_ray_space(sq: Square, blockers: Bitboard) -> Bitboard {
    #[cfg(not(target_feature = "bmi2"))]
    compile_error!("Currently, BMI2 support is required!");

    use std::arch::x86_64::{_pdep_u64, _pext_u64};

    let i = sq.idx() as usize;
    let mut rays = 0u64;
    for [pext, pdep] in PEXT_PDEP[i] {
        rays |= unsafe { _pdep_u64(_pext_u64(blockers.0, pext), pdep) };
    }

    let rev_mask = 0x00000000ffffffff;
    rays = (rays.reverse_bits().swap_bytes() & rev_mask) | (rays & !rev_mask);

    Bitboard(rays)
}
