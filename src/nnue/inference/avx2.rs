use super::*;
use std::arch::x86_64::*;

pub fn forward(us: &[i16; HL], them: &[i16; HL], network: &Network) -> i32 {
    unsafe { forward_impl(us, them, network) }
}

#[target_feature(enable = "avx2")]
fn forward_impl(us: &[i16; HL], them: &[i16; HL], network: &Network) -> i32 {
    assert!(HL.is_multiple_of(16));
    let zero = _mm256_setzero_si256();
    let qa = _mm256_set1_epi16(QA);
    let us_weights = network.out_weight.as_ptr();
    let them_weights = network.out_weight[HL..].as_ptr();

    let mut sums = _mm256_setzero_si256();

    for i in 0..HL / 16 {
        unsafe {
            let us = _mm256_loadu_si256(us.as_ptr().cast::<__m256i>().add(i));
            let weights = _mm256_loadu_si256(us_weights.cast::<__m256i>().add(i));
            let us_clamped = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us));
            sums = _mm256_add_epi32(
                sums,
                _mm256_madd_epi16(us_clamped, _mm256_mullo_epi16(us_clamped, weights)),
            )
        }
    }

    for i in 0..HL / 16 {
        unsafe {
            let them = _mm256_loadu_si256(them.as_ptr().cast::<__m256i>().add(i));
            let weights = _mm256_loadu_si256(them_weights.cast::<__m256i>().add(i));
            let them_clamped = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them));
            sums = _mm256_add_epi32(
                sums,
                _mm256_madd_epi16(them_clamped, _mm256_mullo_epi16(them_clamped, weights)),
            )
        }
    }

    let mut output = reduce_sum(sums);

    output /= i32::from(QA);
    output += i32::from(network.out_bias);

    output *= SCALE;

    output / (i32::from(QA) * i32::from(QB))
}

#[target_feature(enable = "avx2")]
fn reduce_sum(sums: __m256i) -> i32 {
    let sums = _mm_add_epi32(
        _mm256_castsi256_si128(sums),
        _mm256_extracti128_si256(sums, 1),
    );
    let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0xee));
    let sums = _mm_add_epi32(sums, _mm_shuffle_epi32(sums, 0x55));
    _mm_cvtsi128_si32(sums)
}
