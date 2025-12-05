use cfg_if::cfg_if;
use icarus_common::{
    bitboard::Bitboard,
    lookups::{bishop_rays, rook_rays},
    square::Square,
};

cfg_if!(
    if #[cfg(all(target_feature = "avx2", target_feature = "bmi2"))] {
        mod avx2;
        pub use avx2::*;
    } else if #[cfg(target_feature = "bmi2")] {
        mod bmi2;
        pub use bmi2::*;
    }
);

pub fn queen_moves(sq: Square, blockers: Bitboard) -> Bitboard {
    let rays = queen_to_ray_space(sq, blockers).0 | 0x8080808080808080;
    let dst = rays ^ (rays - 0x0101010101010101);
    queen_from_ray_space(sq, Bitboard(dst))
}

pub fn rook_moves(sq: Square, blockers: Bitboard) -> Bitboard {
    queen_moves(sq, blockers) & rook_rays(sq)
}

pub fn bishop_moves(sq: Square, blockers: Bitboard) -> Bitboard {
    queen_moves(sq, blockers) & bishop_rays(sq)
}
