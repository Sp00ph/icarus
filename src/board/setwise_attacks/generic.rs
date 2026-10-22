use crate::{
    board::sliders::{SliderTag, bishop_moves, rook_moves},
    common::{bitboard::Bitboard, lookups::knight_moves},
};

fn knight_attacks_setwise(knights: Bitboard) -> Bitboard {
    knights
        .into_iter()
        .fold(Bitboard::EMPTY, |bb, n| bb | knight_moves(n))
}

fn slider_attacks_setwise(
    orth: Bitboard,
    diag: Bitboard,
    blockers: Bitboard,
    slider_tag: SliderTag,
) -> Bitboard {
    let orth = orth.into_iter().fold(Bitboard::EMPTY, |bb, o| {
        bb | rook_moves(o, blockers, slider_tag)
    });
    let diag = diag.into_iter().fold(Bitboard::EMPTY, |bb, d| {
        bb | bishop_moves(d, blockers, slider_tag)
    });

    orth | diag
}

pub fn knight_and_slider_attacks_setwise(
    knights: Bitboard,
    orth: Bitboard,
    diag: Bitboard,
    blockers: Bitboard,
    slider_tag: SliderTag,
) -> Bitboard {
    knight_attacks_setwise(knights) | slider_attacks_setwise(orth, diag, blockers, slider_tag)
}
