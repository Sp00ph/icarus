#![allow(clippy::missing_safety_doc)]

use std::arch::x86_64::*;

pub type I8Vec = __m256i;
pub type I16Vec = __m256i;
pub type I32Vec = __m256i;

#[cfg(not(all(feature = "use-bmi2", target_feature = "bmi2")))]
#[path = "nnz_table.rs"]
mod nnz_table;

pub mod i8 {
    use super::*;

    pub const LANES: usize = size_of::<I8Vec>() / size_of::<i8>();

    #[target_feature(enable = "avx2")]
    pub unsafe fn load(ptr: *const i8) -> I8Vec {
        unsafe { _mm256_loadu_si256(ptr.cast()) }
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn store(ptr: *mut i8, v: I8Vec) {
        unsafe { _mm256_storeu_si256(ptr.cast(), v) }
    }

    #[target_feature(enable = "avx2")]
    pub fn splat(n: i8) -> I8Vec {
        _mm256_set1_epi8(n)
    }

    #[target_feature(enable = "avx2")]
    pub fn dpbusd(acc: I32Vec, l: I8Vec, r: I8Vec) -> I32Vec {
        cfg_select! {
            target_feature = "avxvnni" => unsafe { _mm256_dpbusd_avx_epi32(acc, l, r) }
            _ => {
                _mm256_add_epi32(
                    acc,
                    _mm256_madd_epi16(
                        _mm256_maddubs_epi16(l, r),
                        _mm256_set1_epi16(1),
                    )
                )
            }
        }
    }

    #[target_feature(enable = "avx2")]
    pub fn reinterpret_i32(v: I8Vec) -> I32Vec {
        v
    }
}

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "avx2")]
    pub unsafe fn load(ptr: *const i16) -> I16Vec {
        unsafe { _mm256_loadu_si256(ptr.cast()) }
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn store(ptr: *mut i16, v: I16Vec) {
        unsafe { _mm256_storeu_si256(ptr.cast(), v) }
    }

    #[target_feature(enable = "avx2")]
    pub fn splat(n: i16) -> I16Vec {
        _mm256_set1_epi16(n)
    }

    #[target_feature(enable = "avx2")]
    pub fn add(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm256_add_epi16(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn min(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm256_min_epi16(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn max(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm256_max_epi16(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn clamp(v: I16Vec, lo: i16, hi: i16) -> I16Vec {
        min(max(v, splat(lo)), splat(hi))
    }

    #[target_feature(enable = "avx2")]
    pub fn mulhi_shl7(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm256_mulhi_epi16(l, _mm256_slli_epi16(r, 7))
    }

    #[target_feature(enable = "avx2")]
    pub fn packus(l: I16Vec, r: I16Vec) -> I8Vec {
        _mm256_permute4x64_epi64(_mm256_packus_epi16(l, r), 0xd8)
    }
}

pub mod i32 {
    use super::*;

    pub const LANES: usize = size_of::<I32Vec>() / size_of::<i32>();

    #[target_feature(enable = "avx2")]
    pub unsafe fn load(ptr: *const i32) -> I32Vec {
        unsafe { _mm256_loadu_si256(ptr.cast()) }
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn store(ptr: *mut i32, v: I32Vec) {
        unsafe { _mm256_storeu_si256(ptr.cast(), v) }
    }

    #[target_feature(enable = "avx2")]
    pub fn splat(n: i32) -> I32Vec {
        _mm256_set1_epi32(n)
    }

    #[target_feature(enable = "avx2")]
    pub fn reinterpret_i8(v: I32Vec) -> I8Vec {
        v
    }

    #[target_feature(enable = "avx2")]
    pub fn add(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm256_add_epi32(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn mul(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm256_mullo_epi32(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn shr_const<const N: i32>(v: I32Vec) -> I32Vec {
        _mm256_srai_epi32(v, N)
    }

    #[target_feature(enable = "avx2")]
    pub fn min(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm256_min_epi32(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn max(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm256_max_epi32(l, r)
    }

    #[target_feature(enable = "avx2")]
    pub fn clamp(v: I32Vec, lo: i32, hi: i32) -> I32Vec {
        min(max(v, splat(lo)), splat(hi))
    }

    #[target_feature(enable = "avx2")]
    pub fn reduce_sum(v: I32Vec) -> i32 {
        let sum128 = _mm_add_epi32(_mm256_castsi256_si128(v), _mm256_extracti128_si256(v, 1));
        let sum64 = _mm_add_epi32(sum128, _mm_shuffle_epi32(sum128, 0xee));
        let sum32 = _mm_add_epi32(sum64, _mm_shuffle_epi32(sum64, 0x55));
        _mm_cvtsi128_si32(sum32)
    }

    #[target_feature(enable = "avx2")]
    pub fn nnz_indices(v: I32Vec) -> (I16Vec, u16) {
        let nnz_mask = _mm256_movemask_ps(_mm256_castsi256_ps(_mm256_cmpgt_epi32(v, splat(0))));

        let idxs = cfg_select! {
            all(feature = "use-bmi2", target_feature = "bmi2") => unsafe {
                let mask = _pdep_u64(nnz_mask as u64, 0x0101010101010101) * 255;
                let idxs = _pext_u64(0x0706050403020100, mask);
                _mm_cvtepi8_epi16(_mm_cvtsi64_si128(idxs as i64))
            }
            _ => unsafe { _mm_loadu_si128(nnz_table::NNZ_TABLE[nnz_mask as usize].as_ptr().cast()) }
        };

        let count = nnz_mask.count_ones();
        (_mm256_castsi128_si256(idxs), count as u16)
    }
}
