use arrayvec::ArrayVec;
use icarus_common::{
    bitboard::Bitboard,
    piece::{Color, Piece},
    square::Square,
    util::enum_map::EnumMap,
};

use crate::nnue::network::{HL, INPUT, NET, NUM_KING_BUCKETS, should_mirror};

#[derive(Debug, Clone)]
pub struct Accumulator {
    pub values: EnumMap<Color, [i16; HL]>,
    pub dirty: EnumMap<Color, bool>,
    pub needs_refresh: EnumMap<Color, bool>,
    pub updates: Updates,
}

#[derive(Clone, Copy, Debug)]
pub struct Feature {
    pub piece: Piece,
    pub color: Color,
    pub square: Square,
}

impl Feature {
    pub fn idx(&self, perspective: Color, king: Square) -> usize {
        let (mut square, mut color) = match perspective {
            Color::White => (self.square, self.color),
            Color::Black => (self.square.flip_rank(), !self.color),
        };

        if should_mirror(king) {
            square = square.flip_file();
        }
        if self.piece == Piece::King {
            color = Color::White;
        }

        square as usize + Square::COUNT * (self.piece as usize + Piece::COUNT * color as usize)
    }
}

#[derive(Clone, Default, Debug)]
pub struct Updates {
    pub adds: ArrayVec<Feature, 2>,
    pub subs: ArrayVec<Feature, 2>,
}

impl Updates {
    pub fn move_piece(&mut self, from: Square, to: Square, piece: Piece, color: Color) {
        self.remove_piece(from, piece, color);
        self.add_piece(to, piece, color);
    }

    pub fn add_piece(&mut self, square: Square, piece: Piece, color: Color) {
        self.adds.push(Feature {
            piece,
            color,
            square,
        })
    }

    pub fn remove_piece(&mut self, square: Square, piece: Piece, color: Color) {
        self.subs.push(Feature {
            piece,
            color,
            square,
        });
    }
}

#[repr(align(64))]
#[derive(Default)]
/// An accumulator cache for refreshing on king bucket changes, also known as "Finny tables".
pub struct KingBucketCache {
    /// indexed by [stm][mirror][bucket]
    pub entries: [[[Entry; NUM_KING_BUCKETS]; 2]; 2],
}

pub struct Entry {
    pub features: [i16; HL],
    pub pieces: EnumMap<Piece, Bitboard>,
    pub colors: EnumMap<Color, Bitboard>,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            features: NET.ft_bias,
            pieces: Default::default(),
            colors: Default::default(),
        }
    }
}

pub fn acc_add(acc: &mut [i16; HL], weights: &[[i16; HL]; INPUT], add: usize) {
    for (acc, weight) in acc.iter_mut().zip(&weights[add]) {
        *acc += *weight;
    }
}

pub fn acc_sub(acc: &mut [i16; HL], weights: &[[i16; HL]; INPUT], sub: usize) {
    for (acc, weight) in acc.iter_mut().zip(&weights[sub]) {
        *acc -= *weight;
    }
}

pub fn acc_add_sub(
    src: &[i16; HL],
    dst: &mut [i16; HL],
    weights: &[[i16; HL]; INPUT],
    add: usize,
    sub: usize,
) {
    let add: &[i16; HL] = &weights[add];
    let sub: &[i16; HL] = &weights[sub];

    for i in 0..HL {
        dst[i] = src[i] + add[i] - sub[i];
    }
}

pub fn acc_add_sub2(
    src: &[i16; HL],
    dst: &mut [i16; HL],
    weights: &[[i16; HL]; INPUT],
    add: usize,
    sub1: usize,
    sub2: usize,
) {
    let add: &[i16; HL] = &weights[add];
    let sub1: &[i16; HL] = &weights[sub1];
    let sub2: &[i16; HL] = &weights[sub2];

    for i in 0..HL {
        dst[i] = src[i] + add[i] - sub1[i] - sub2[i];
    }
}

pub fn acc_add2_sub2(
    src: &[i16; HL],
    dst: &mut [i16; HL],
    weights: &[[i16; HL]; INPUT],
    add1: usize,
    add2: usize,
    sub1: usize,
    sub2: usize,
) {
    let add1: &[i16; HL] = &weights[add1];
    let add2: &[i16; HL] = &weights[add2];
    let sub1: &[i16; HL] = &weights[sub1];
    let sub2: &[i16; HL] = &weights[sub2];

    for i in 0..HL {
        dst[i] = src[i] + add1[i] + add2[i] - sub1[i] - sub2[i];
    }
}

pub fn acc_add4(
    dst: &mut [i16; HL],
    weights: &[[i16; HL]; INPUT],
    add1: usize,
    add2: usize,
    add3: usize,
    add4: usize,
) {
    let add1: &[i16; HL] = &weights[add1];
    let add2: &[i16; HL] = &weights[add2];
    let add3: &[i16; HL] = &weights[add3];
    let add4: &[i16; HL] = &weights[add4];

    for i in 0..HL {
        dst[i] += add1[i] + add2[i] + add3[i] + add4[i];
    }
}

pub fn acc_sub4(
    dst: &mut [i16; HL],
    weights: &[[i16; HL]; INPUT],
    sub1: usize,
    sub2: usize,
    sub3: usize,
    sub4: usize,
) {
    let sub1: &[i16; HL] = &weights[sub1];
    let sub2: &[i16; HL] = &weights[sub2];
    let sub3: &[i16; HL] = &weights[sub3];
    let sub4: &[i16; HL] = &weights[sub4];

    for i in 0..HL {
        dst[i] += -sub1[i] - sub2[i] - sub3[i] - sub4[i];
    }
}
