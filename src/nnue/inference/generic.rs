use super::*;

pub fn forward(us: &[i16; HL], them: &[i16; HL], bucket: usize) -> i32 {
    debug_assert!(bucket < OUT_BUCKETS);

    let mut output = 0;

    let us_weights: &[i16; HL] = &NET.out_weight[bucket][0];
    let them_weights: &[i16; HL] = &NET.out_weight[bucket][1];

    for (&us, &weight) in us.iter().zip(us_weights) {
        let us_clamped = us.clamp(0, QA);
        output += i32::from(us_clamped * weight) * i32::from(us_clamped);
    }

    for (&them, &weight) in them.iter().zip(them_weights) {
        let them_clamped = them.clamp(0, QA);
        output += i32::from(them_clamped * weight) * i32::from(them_clamped);
    }

    output /= i32::from(QA);
    output += i32::from(NET.out_bias[bucket]);

    output *= SCALE;

    output / (i32::from(QA) * i32::from(QB))
}
