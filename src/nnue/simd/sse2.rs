#![allow(clippy::missing_safety_doc)]

use std::arch::x86_64::*;

pub type I8Vec = __m128i;
pub type I16Vec = __m128i;
pub type I32Vec = __m128i;

#[cfg(not(all(feature = "use-bmi2", target_feature = "bmi2")))]
#[path = "nnz_table.rs"]
mod nnz_table;

pub mod i8 {
    use super::*;

    pub const LANES: usize = size_of::<I8Vec>() / size_of::<i8>();

    #[target_feature(enable = "sse2")]
    pub unsafe fn load(ptr: *const i8) -> I8Vec {
        unsafe { _mm_loadu_si128(ptr.cast()) }
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn store(ptr: *mut i8, v: I8Vec) {
        unsafe { _mm_storeu_si128(ptr.cast(), v) }
    }

    #[target_feature(enable = "sse2")]
    pub fn splat(n: i8) -> I8Vec {
        _mm_set1_epi8(n)
    }

    #[target_feature(enable = "sse2")]
    pub fn dpbusd(acc: I32Vec, l: I8Vec, r: I8Vec) -> I32Vec {
        _mm_add_epi32(
            acc,
            _mm_madd_epi16(
                _mm_set1_epi16(1),
                cfg_select! {
                    target_feature = "ssse3" => unsafe { _mm_maddubs_epi16(l, r) }
                    _ => {
                        _mm_add_epi16(
                            _mm_mullo_epi16(
                                _mm_and_si128(l, _mm_set1_epi16(255)),
                                _mm_srai_epi16(_mm_slli_epi16(r, 8), 8),
                            ),
                            _mm_mullo_epi16(
                                _mm_srli_epi16(l, 8),
                                _mm_srai_epi16(r, 8),
                            ),
                        )
                    }
                },
            ),
        )
    }

    #[target_feature(enable = "sse2")]
    pub fn reinterpret_i32(v: I8Vec) -> I32Vec {
        v
    }
}

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "sse2")]
    pub unsafe fn load(ptr: *const i16) -> I16Vec {
        unsafe { _mm_loadu_si128(ptr.cast()) }
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn store(ptr: *mut i16, v: I16Vec) {
        unsafe { _mm_storeu_si128(ptr.cast(), v) }
    }

    #[target_feature(enable = "sse2")]
    pub fn splat(n: i16) -> I16Vec {
        _mm_set1_epi16(n)
    }

    #[target_feature(enable = "sse2")]
    pub fn add(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm_add_epi16(l, r)
    }

    #[target_feature(enable = "sse2")]
    pub fn min(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm_min_epi16(l, r)
    }

    #[target_feature(enable = "sse2")]
    pub fn max(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm_max_epi16(l, r)
    }

    #[target_feature(enable = "sse2")]
    pub fn mulhi_shl7(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm_mulhi_epi16(l, _mm_slli_epi16(r, 7))
    }

    #[target_feature(enable = "sse2")]
    pub fn packus(l: I16Vec, r: I16Vec) -> I8Vec {
        _mm_packus_epi16(l, r)
    }
}

pub mod i32 {
    use super::*;

    pub const LANES: usize = size_of::<I32Vec>() / size_of::<i32>();

    #[target_feature(enable = "sse2")]
    pub unsafe fn load(ptr: *const i32) -> I32Vec {
        unsafe { _mm_loadu_si128(ptr.cast()) }
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn store(ptr: *mut i32, v: I32Vec) {
        unsafe { _mm_storeu_si128(ptr.cast(), v) }
    }

    #[target_feature(enable = "sse2")]
    pub fn splat(n: i32) -> I32Vec {
        _mm_set1_epi32(n)
    }

    #[target_feature(enable = "sse2")]
    pub fn reinterpret_i8(v: I32Vec) -> I8Vec {
        v
    }

    #[target_feature(enable = "sse2")]
    pub fn add(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm_add_epi32(l, r)
    }

    #[target_feature(enable = "sse2")]
    pub fn mul(l: I32Vec, r: I32Vec) -> I32Vec {
        cfg_select! {
            target_feature = "sse4.1" => unsafe { _mm_mullo_epi32(l, r) },
            _ => {
                _mm_unpacklo_epi32(
                    _mm_shuffle_epi32(
                        _mm_mul_epu32(l, r),
                        232,
                    ),
                    _mm_shuffle_epi32(
                        _mm_mul_epu32(
                            _mm_shuffle_epi32(l, 245),
                            _mm_shuffle_epi32(r, 245),
                        ), 
                        232
                    )
                )
            }
        }
    }

    #[target_feature(enable = "sse2")]
    pub fn shr_const<const N: i32>(v: I32Vec) -> I32Vec {
        _mm_srai_epi32(v, N)
    }

    #[target_feature(enable = "sse2")]
    pub fn min(l: I32Vec, r: I32Vec) -> I32Vec {
        cfg_select! {
            target_feature = "sse4.1" => unsafe { _mm_min_epi32(l, r) },
            _ => {
                let mask = _mm_cmpgt_epi32(l, r);
                _mm_or_si128(
                    _mm_and_si128(mask, r),
                    _mm_andnot_si128(mask, l)
                )
            }
        }
    }

    #[target_feature(enable = "sse2")]
    pub fn max(l: I32Vec, r: I32Vec) -> I32Vec {
        cfg_select! {
            target_feature = "sse4.1" => unsafe { _mm_max_epi32(l, r) },
            _ => {
                let mask = _mm_cmpgt_epi32(l, r);
                _mm_or_si128(
                    _mm_and_si128(mask, l),
                    _mm_andnot_si128(mask, r)
                )
            }
        }
    }

    #[target_feature(enable = "sse2")]
    pub fn reduce_sum(v: I32Vec) -> i32 {
        let sum64 = _mm_add_epi32(v, _mm_shuffle_epi32(v, 0xee));
        let sum32 = _mm_add_epi32(sum64, _mm_shuffle_epi32(sum64, 0x55));
        _mm_cvtsi128_si32(sum32)
    }

    #[target_feature(enable = "sse2")]
    pub fn nnz_indices(v: I32Vec) -> (I16Vec, u16) {
        let nnz_mask = _mm_movemask_ps(_mm_castsi128_ps(_mm_cmpgt_epi32(v, splat(0))));

        let idxs = cfg_select! {
            all(feature = "use-bmi2", target_feature = "bmi2") => unsafe {
                let mask = _pdep_u64(nnz_mask as u64, 0x0101010101010101) * 255;
                let idxs = _pext_u64(0x0706050403020100, mask);
                _mm_cvtepi8_epi16(_mm_cvtsi64_si128(idxs as i64))
            }
            _ => unsafe { _mm_loadu_si128(nnz_table::NNZ_TABLE[nnz_mask as usize].as_ptr().cast()) }
        };

        let count = nnz_mask.count_ones();
        (idxs, count as u16)
    }
}
