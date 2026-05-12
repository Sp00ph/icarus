#![allow(clippy::missing_safety_doc)]
use std::arch::x86_64::*;

pub type I8Vec = __m512i;
pub type I16Vec = __m512i;
pub type I32Vec = __m512i;

pub mod i8 {
    use super::*;

    pub const LANES: usize = size_of::<I8Vec>() / size_of::<i8>();

    #[target_feature(enable = "avx512f")]
    pub unsafe fn load(ptr: *const i8) -> I8Vec {
        unsafe { _mm512_loadu_si512(ptr.cast()) }
    }

    #[target_feature(enable = "avx512f")]
    pub unsafe fn store(ptr: *mut i8, v: I8Vec) {
        unsafe { _mm512_storeu_si512(ptr.cast(), v) }
    }

    #[target_feature(enable = "avx512f")]
    pub fn splat(n: i8) -> I8Vec {
        _mm512_set1_epi8(n)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn dpbusd(acc: I32Vec, l: I8Vec, r: I8Vec) -> I32Vec {
        cfg_select! {
            target_feature = "avx512vnni" => unsafe { _mm512_dpbusd_epi32(acc, l, r) }
            _ => {
                _mm512_add_epi32(
                    acc,
                    _mm512_madd_epi16(
                        _mm512_maddubs_epi16(l, r),
                        _mm512_set1_epi16(1),
                    )
                )
            }
        }
    }

    #[target_feature(enable = "avx512f")]
    pub fn reinterpret_i32(v: I8Vec) -> I32Vec {
        v
    }
}

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "avx512f")]
    pub unsafe fn load(ptr: *const i16) -> I16Vec {
        unsafe { _mm512_loadu_si512(ptr.cast()) }
    }

    #[target_feature(enable = "avx512f")]
    pub unsafe fn store(ptr: *mut i16, v: I16Vec) {
        unsafe { _mm512_storeu_si512(ptr.cast(), v) }
    }

    #[target_feature(enable = "avx512f")]
    pub fn splat(n: i16) -> I16Vec {
        _mm512_set1_epi16(n)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn add(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm512_add_epi16(l, r)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn min(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm512_min_epi16(l, r)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn max(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm512_max_epi16(l, r)
    }

    #[target_feature(enable = "avx512bw")]
    pub fn mulhi_shl7(l: I16Vec, r: I16Vec) -> I16Vec {
        _mm512_mulhi_epi16(l, _mm512_slli_epi16(r, 7))
    }

    #[target_feature(enable = "avx512bw")]
    pub fn packus(l: I16Vec, r: I16Vec) -> I8Vec {
        let lo = _mm512_shuffle_i64x2(l, r, 136);
        let hi = _mm512_shuffle_i64x2(l, r, 221);
        _mm512_packus_epi16(lo, hi)
    }
}

pub mod i32 {
    use super::*;

    pub const LANES: usize = size_of::<I32Vec>() / size_of::<i32>();

    #[target_feature(enable = "avx512f")]
    pub unsafe fn load(ptr: *const i32) -> I32Vec {
        unsafe { _mm512_loadu_si512(ptr.cast()) }
    }

    #[target_feature(enable = "avx512f")]
    pub unsafe fn store(ptr: *mut i32, v: I32Vec) {
        unsafe { _mm512_storeu_si512(ptr.cast(), v) }
    }

    #[target_feature(enable = "avx512f")]
    pub fn splat(n: i32) -> I32Vec {
        _mm512_set1_epi32(n)
    }

    #[target_feature(enable = "avx512f")]
    pub fn reinterpret_i8(v: I32Vec) -> I8Vec {
        v
    }

    #[target_feature(enable = "avx512f")]
    pub fn add(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm512_add_epi32(l, r)
    }

    #[target_feature(enable = "avx512f")]
    pub fn mul(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm512_mullo_epi32(l, r)
    }

    #[target_feature(enable = "avx512f")]
    pub fn shr_const<const N: u32>(v: I32Vec) -> I32Vec {
        _mm512_srai_epi32(v, N)
    }

    #[target_feature(enable = "avx512f")]
    pub fn min(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm512_min_epi32(l, r)
    }

    #[target_feature(enable = "avx512f")]
    pub fn max(l: I32Vec, r: I32Vec) -> I32Vec {
        _mm512_max_epi32(l, r)
    }

    #[target_feature(enable = "avx512f")]
    pub fn reduce_sum(v: I32Vec) -> i32 {
        _mm512_reduce_add_epi32(v)
    }

    #[target_feature(enable = "avx512vbmi2,avx512vl")]
    pub fn nnz_indices(v: I32Vec) -> (I16Vec, u16) {
        let nnz_mask = _mm512_test_epi32_mask(v, v);
        let idxs: [i16; 16] = std::array::from_fn(|i| i as i16);
        let idxs = unsafe { _mm256_loadu_si256(idxs.as_ptr().cast()) };
        (
            _mm512_castsi256_si512(_mm256_maskz_compress_epi16(nnz_mask, idxs)),
            nnz_mask.count_ones() as u16,
        )
    }
}
