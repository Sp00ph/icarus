use super::*;

pub fn forward(us: &[i16; HL], them: &[i16; HL]) -> i32 {
    let mut output = 0;

    for (&us, &weight) in us.iter().zip(&NET.out_weight[..HL]) {
        let us_clamped = us.clamp(0, QA);
        output += i32::from(us_clamped * weight) * i32::from(us_clamped);
    }

    for (&them, &weight) in them.iter().zip(&NET.out_weight[HL..]) {
        let them_clamped = them.clamp(0, QA);
        output += i32::from(them_clamped * weight) * i32::from(them_clamped);
    }

    output /= i32::from(QA);
    output += i32::from(NET.out_bias);

    output *= SCALE;

    output / (i32::from(QA) * i32::from(QB))
}
