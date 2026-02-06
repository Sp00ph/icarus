use super::*;
use std::arch::x86_64::*;

pub fn forward(us: &[i16; HL], them: &[i16; HL], network: &Network) -> i32 {
    unsafe { forward_impl(us, them, network) }
}

#[target_feature(enable = "avx2")]
fn forward_impl(us: &[i16; HL], them: &[i16; HL], network: &Network) -> i32 {
    assert!(HL.is_multiple_of(64));
    let zero = _mm256_setzero_si256();
    let qa = _mm256_set1_epi16(QA);
    let us_weights = network.out_weight.as_ptr();
    let them_weights = network.out_weight[HL..].as_ptr();

    let mut sums0 = _mm256_setzero_si256();
    let mut sums1 = _mm256_setzero_si256();
    let mut sums2 = _mm256_setzero_si256();
    let mut sums3 = _mm256_setzero_si256();

    for i in 0..HL / 64 {
        unsafe {
            let us0 = _mm256_loadu_si256(us.as_ptr().cast::<__m256i>().add(4 * i + 0));
            let us1 = _mm256_loadu_si256(us.as_ptr().cast::<__m256i>().add(4 * i + 1));
            let us2 = _mm256_loadu_si256(us.as_ptr().cast::<__m256i>().add(4 * i + 2));
            let us3 = _mm256_loadu_si256(us.as_ptr().cast::<__m256i>().add(4 * i + 3));
            let weights0 = _mm256_loadu_si256(us_weights.cast::<__m256i>().add(4 * i + 0));
            let weights1 = _mm256_loadu_si256(us_weights.cast::<__m256i>().add(4 * i + 1));
            let weights2 = _mm256_loadu_si256(us_weights.cast::<__m256i>().add(4 * i + 2));
            let weights3 = _mm256_loadu_si256(us_weights.cast::<__m256i>().add(4 * i + 3));
            let us_clamped0 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us0));
            let us_clamped1 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us1));
            let us_clamped2 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us2));
            let us_clamped3 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, us3));
            sums0 = _mm256_add_epi32(
                sums0,
                _mm256_madd_epi16(us_clamped0, _mm256_mullo_epi16(us_clamped0, weights0)),
            );
            sums1 = _mm256_add_epi32(
                sums1,
                _mm256_madd_epi16(us_clamped1, _mm256_mullo_epi16(us_clamped1, weights1)),
            );
            sums2 = _mm256_add_epi32(
                sums2,
                _mm256_madd_epi16(us_clamped2, _mm256_mullo_epi16(us_clamped2, weights2)),
            );
            sums3 = _mm256_add_epi32(
                sums3,
                _mm256_madd_epi16(us_clamped3, _mm256_mullo_epi16(us_clamped3, weights3)),
            );
        }
    }

    for i in 0..HL / 64 {
        unsafe {
            let them0 = _mm256_loadu_si256(them.as_ptr().cast::<__m256i>().add(4 * i + 0));
            let them1 = _mm256_loadu_si256(them.as_ptr().cast::<__m256i>().add(4 * i + 1));
            let them2 = _mm256_loadu_si256(them.as_ptr().cast::<__m256i>().add(4 * i + 2));
            let them3 = _mm256_loadu_si256(them.as_ptr().cast::<__m256i>().add(4 * i + 3));
            let weights0 = _mm256_loadu_si256(them_weights.cast::<__m256i>().add(4 * i + 0));
            let weights1 = _mm256_loadu_si256(them_weights.cast::<__m256i>().add(4 * i + 1));
            let weights2 = _mm256_loadu_si256(them_weights.cast::<__m256i>().add(4 * i + 2));
            let weights3 = _mm256_loadu_si256(them_weights.cast::<__m256i>().add(4 * i + 3));
            let them_clamped0 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them0));
            let them_clamped1 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them1));
            let them_clamped2 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them2));
            let them_clamped3 = _mm256_max_epi16(zero, _mm256_min_epi16(qa, them3));
            sums0 = _mm256_add_epi32(
                sums0,
                _mm256_madd_epi16(them_clamped0, _mm256_mullo_epi16(them_clamped0, weights0)),
            );
            sums1 = _mm256_add_epi32(
                sums1,
                _mm256_madd_epi16(them_clamped1, _mm256_mullo_epi16(them_clamped1, weights1)),
            );
            sums2 = _mm256_add_epi32(
                sums2,
                _mm256_madd_epi16(them_clamped2, _mm256_mullo_epi16(them_clamped2, weights2)),
            );
            sums3 = _mm256_add_epi32(
                sums3,
                _mm256_madd_epi16(them_clamped3, _mm256_mullo_epi16(them_clamped3, weights3)),
            );
        }
    }

    let sums = _mm256_add_epi32(
        _mm256_add_epi32(sums0, sums1),
        _mm256_add_epi32(sums2, sums3),
    );

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
