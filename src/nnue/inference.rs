use crate::nnue::network::{Network, OUT_BUCKETS};

use super::network::HL;

const QA: i16 = 255;
const QB: i16 = 64;

const SCALE: i32 = 400;

// TODO: simd this

pub fn forward(us: &[i16; HL], them: &[i16; HL], network: &Network, bucket: usize) -> i32 {
    debug_assert!(bucket < OUT_BUCKETS);

    let us_weights: &[i16; HL] = network.out_weight[bucket * 2 * HL..][..HL]
        .try_into()
        .unwrap();
    let them_weights: &[i16; HL] = network.out_weight[(bucket * 2 + 1) * HL..][..HL]
        .try_into()
        .unwrap();

    let mut output = 0;

    for (&us, &weight) in us.iter().zip(us_weights) {
        let us_clamped = us.clamp(0, QA);
        output += i32::from(us_clamped * weight) * i32::from(us_clamped);
    }

    for (&them, &weight) in them.iter().zip(them_weights) {
        let them_clamped = them.clamp(0, QA);
        output += i32::from(them_clamped * weight) * i32::from(them_clamped);
    }

    output /= i32::from(QA);
    output += i32::from(network.out_bias[bucket]);

    output *= SCALE;

    output / (i32::from(QA) * i32::from(QB))
}
