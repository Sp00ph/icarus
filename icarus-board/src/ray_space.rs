use icarus_common::{
    bitboard::Bitboard,
    square::Square,
};

static FILE_PEXT_PDEP: [[u64; 64]; 2] = {
    let mut pext = [0u64; 64];
    let mut pdep = [0u64; 64];
    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        let rank = sq.rank().idx() as u32;

        let n_south = rank;
        let n_north = 7 - rank;

        pext[i] = sq.file().bitboard().subtract(sq.bitboard()).0;
        pdep[i] = (((1u64 << n_south) - 1) << (8 - n_south)) | (((1u64 << n_north) - 1) << 32);

        i += 1;
    }
    [pext, pdep]
};

static RANK_PEXT_PDEP: [[u64; 64]; 2] = {
    let mut pext = [0u64; 64];
    let mut pdep = [0u64; 64];
    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        let file = sq.file().idx() as u32;

        let n_west = file;
        let n_east = 7 - file;

        pext[i] = sq.rank().bitboard().subtract(sq.bitboard()).0;
        pdep[i] = (((1u64 << n_west) - 1) << (24 - n_west)) | (((1u64 << n_east) - 1) << 48);

        i += 1;
    }

    [pext, pdep]
};

static MAIN_DIAG_PEXT_PDEP: [[u64; 64]; 2] = {
    let mut pext = [0u64; 64];
    let mut pdep = [0u64; 64];
    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        let (file, rank) = (sq.file().idx() as u32, sq.rank().idx() as u32);

        let n_se = if rank < 7 - file { rank } else { 7 - file };
        let n_nw = if file < 7 - rank { file } else { 7 - rank };
        pext[i] = Bitboard::main_diag_for(sq).subtract(sq.bitboard()).0;
        pdep[i] = (((1u64 << n_se) - 1) << (16 - n_se)) | (((1u64 << n_nw) - 1) << 40);

        i += 1;
    }
    [pext, pdep]
};

static ANTI_DIAG_PEXT_PDEP: [[u64; 64]; 2] = {
    let mut pext = [0u64; 64];
    let mut pdep = [0u64; 64];
    let mut i = 0;
    while i < 64 {
        let sq = Square::from_idx(i as u8);
        let (file, rank) = (sq.file().idx() as u32, sq.rank().idx() as u32);

        let n_sw = if rank < file { rank } else { file };
        let n_ne = if file > rank { 7 - file } else { 7 - rank };
        pext[i] = Bitboard::anti_diag_for(sq).subtract(sq.bitboard()).0;
        pdep[i] = (((1u64 << n_sw) - 1) << (16 - n_sw)) | (((1u64 << n_ne) - 1) << 40);

        i += 1;
    }
    [pext, pdep]
};

/// Convert a blocker bitboard into ray space format.
/// Each byte corresponds to one ray, in one of the 8 compass directions.
/// From least to most significant byte, they are ordered S, SW, W, NW, N, NE, E, SE.
/// Within each byte, the least significant bit corresponds to the square closest to
/// the source.
pub fn queen_rays(sq: Square, blockers: Bitboard) -> Bitboard {
    #[cfg(not(target_feature = "bmi2"))]
    compile_error!("Currently, BMI2 support is required!");

    use std::arch::x86_64::{_pdep_u64, _pext_u64};

    let i = sq.idx() as usize;
    let mut rays = 0u64;
    for [pext, pdep] in [
        &FILE_PEXT_PDEP,
        &RANK_PEXT_PDEP,
        &MAIN_DIAG_PEXT_PDEP,
        &ANTI_DIAG_PEXT_PDEP,
    ] {
        rays |= unsafe { _pdep_u64(_pext_u64(blockers.0, pext[i]), pdep[i]) };
    }

    let rev_mask = 0xff00000000ffffffu64;
    rays = (rays.swap_bytes().reverse_bits() & rev_mask) | (rays & !rev_mask);

    Bitboard(rays)
}
