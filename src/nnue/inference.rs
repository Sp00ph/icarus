use crate::nnue::network::Network;

use super::network::HL;

const QA: i16 = 255;
const QB: i16 = 64;

const SCALE: i32 = 400;

// TODO: simd this

pub fn forward(us: &[i16; HL], them: &[i16; HL], network: &Network) -> i32 {
    let mut output = 0;

    for (&us, &weight) in us.iter().zip(&network.out_weight[..HL]) {
        let us_clamped = us.clamp(0, QA);
        output += i32::from(us_clamped * weight) * i32::from(us_clamped);
    }

    for (&them, &weight) in them.iter().zip(&network.out_weight[HL..]) {
        let them_clamped = them.clamp(0, QA);
        output += i32::from(them_clamped * weight) * i32::from(them_clamped);
    }

    output /= i32::from(QA);
    output += i32::from(network.out_bias);

    output *= SCALE;

    output / (i32::from(QA) * i32::from(QB))
}
