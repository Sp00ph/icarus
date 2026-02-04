use arrayvec::ArrayVec;
use icarus_common::{
    piece::{Color, Piece},
    square::Square,
    util::enum_map::EnumMap,
};

use crate::nnue::network::{HL, Network};

#[derive(Debug, Clone)]
pub struct Accumulator {
    pub values: EnumMap<Color, [i16; HL]>,
    pub dirty: EnumMap<Color, bool>,
    pub updates: Updates,
}

#[derive(Clone, Copy, Debug)]
pub struct Feature {
    pub piece: Piece,
    pub color: Color,
    pub square: Square,
}

impl Feature {
    pub fn idx(&self, perspective: Color) -> usize {
        let (square, color) = match perspective {
            Color::White => (self.square, self.color),
            Color::Black => (self.square.flip_rank(), !self.color),
        };

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

// TODO: simd these

pub fn acc_add(acc: &mut [i16; HL], network: &Network, adds: &[usize]) {
    for &add in adds {
        for (acc, weight) in acc.iter_mut().zip(&network.ft_weight[add * HL..]) {
            *acc += *weight;
        }
    }
}

pub fn acc_add_sub(
    src: &[i16; HL],
    dst: &mut [i16; HL],
    network: &Network,
    add: usize,
    sub: usize,
) {
    let add: &[i16; HL] = network.ft_weight[add * HL..][..HL].try_into().unwrap();
    let sub: &[i16; HL] = network.ft_weight[sub * HL..][..HL].try_into().unwrap();

    for i in 0..HL {
        dst[i] = src[i] + add[i] - sub[i];
    }
}

pub fn acc_add_sub2(
    src: &[i16; HL],
    dst: &mut [i16; HL],
    network: &Network,
    add: usize,
    sub1: usize,
    sub2: usize,
) {
    let add: &[i16; HL] = network.ft_weight[add * HL..][..HL].try_into().unwrap();
    let sub1: &[i16; HL] = network.ft_weight[sub1 * HL..][..HL].try_into().unwrap();
    let sub2: &[i16; HL] = network.ft_weight[sub2 * HL..][..HL].try_into().unwrap();

    for i in 0..HL {
        dst[i] = src[i] + add[i] - sub1[i] - sub2[i];
    }
}

pub fn acc_add2_sub2(
    src: &[i16; HL],
    dst: &mut [i16; HL],
    network: &Network,
    add1: usize,
    add2: usize,
    sub1: usize,
    sub2: usize,
) {
    let add1: &[i16; HL] = network.ft_weight[add1 * HL..][..HL].try_into().unwrap();
    let add2: &[i16; HL] = network.ft_weight[add2 * HL..][..HL].try_into().unwrap();
    let sub1: &[i16; HL] = network.ft_weight[sub1 * HL..][..HL].try_into().unwrap();
    let sub2: &[i16; HL] = network.ft_weight[sub2 * HL..][..HL].try_into().unwrap();

    for i in 0..HL {
        dst[i] = src[i] + add1[i] + add2[i] - sub1[i] - sub2[i];
    }
}
