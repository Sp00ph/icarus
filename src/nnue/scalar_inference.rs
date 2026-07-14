// Mostly here to debug the SIMD inference.

use crate::nnue::network::{L1, L2, L3, NET};

const Q0: i16 = 255;
const _Q1: i16 = 128;
const Q: i32 = 64;
const SCALE: i32 = 400;

fn activate_ft(us: &[i16; L1], them: &[i16; L1]) -> [i8; L1] {
    let mut out = [0i8; L1];

    for (i, acc) in [us, them].into_iter().enumerate() {
        let base = L1 / 2 * i;

        for j in 0..L1 / 2 {
            out[base + j] = ((acc[j].clamp(0, Q0) as u16 * acc[j + L1 / 2].clamp(0, Q0) as u16) >> 9) as i8;
        }
    }

    out
}

fn propagate_l1(input: &[i8; L1]) -> [i32; L2] {
    let mut intermediate = [0i32; L2];

    for output_idx in 0..L2 {
        for input_idx in 0..L1 {
            let in_block = input_idx / 4;
            let k = input_idx % 4;
            let weight: i32 = NET.l1w[in_block][output_idx * 4 + k] as i32;
            intermediate[output_idx] += input[input_idx] as i32 * weight;
        }
    }

    let mut output = [0i32; L2];

    for i in 0..L2 {
        let bias = NET.l1b[i];
        output[i] = ((intermediate[i] >> 8) + bias).clamp(0, Q).pow(2);
    }

    output
}

fn propagate_l2(input: &[i32; L2]) -> [i32; L3] {
    let mut out = NET.l2b;

    for input_idx in 0..L2 {
        let input_val = input[input_idx];
        for output_idx in 0..L3 {
            let weight = NET.l2w[input_idx][output_idx];
            out[output_idx] += input_val * weight;
        }
    }

    out
}

fn propagate_l3(input: &[i32; L3]) -> i32 {
    let mut output: i32 = NET.l3b;
    for (&input, &weight) in input.iter().zip(NET.l3w.iter()) {
        let clamped = input.clamp(0, (Q * Q * Q) as i32);
        // This multiplication moves us into [0, Q^4] space
        output += clamped * weight;
    }
    output
}


#[inline(never)]
pub fn forward(us: &[i16; L1], them: &[i16; L1]) -> i32 {
    // in [0, Q1]
    let act_ft = activate_ft(us, them);
    // in [0, Q^2]
    let act_l1 = propagate_l1(&act_ft);
    // in [0, Q^3]
    let act_l2 = propagate_l2(&act_l1);
    // in [0, SCALE * Q^4]
    let scaled = propagate_l3(&act_l2) as i64 * SCALE as i64;

    (scaled / (Q.pow(4) as i64)) as i32
}
