#![allow(clippy::missing_safety_doc)]

#[path = "nnz_table.rs"]
mod nnz_table;

use std::arch::aarch64::*;

pub type I8Vec = int8x16_t;
pub type I16Vec = int16x8_t;
pub type I32Vec = int32x4_t;

// Neon doesn't have fused load adds, so we only
// use half the registers to keep the other half
// available for loads.
pub const NUM_ACC_REGS: usize = 8;

pub mod i8 {
    use super::*;

    pub const LANES: usize = size_of::<I8Vec>() / size_of::<i8>();

    #[target_feature(enable = "neon")]
    pub unsafe fn load(ptr: *const i8) -> I8Vec {
        unsafe { vld1q_s8(ptr) }
    }

    #[target_feature(enable = "neon")]
    pub unsafe fn store(ptr: *mut i8, v: I8Vec) {
        unsafe { vst1q_s8(ptr, v) }
    }

    #[target_feature(enable = "neon")]
    pub fn splat(n: i8) -> I8Vec {
        vdupq_n_s8(n)
    }

    #[target_feature(enable = "neon")]
    pub fn dpbusd(acc: I32Vec, l: I8Vec, r: I8Vec) -> I32Vec {
        cfg_select! {
            target_feature = "dotprod" => unsafe {
                let mut acc = acc;
                std::arch::asm!(
                    "sdot {acc:v}.4s, {l:v}.16b, {r:v}.16b",
                    acc = inlateout(vreg) acc,
                    l = in(vreg) l,
                    r = in(vreg) r,
                    options(pure, nostack, nomem, preserves_flags)
                );
                acc
            }
            _ => {
                let lo = vmull_s8(vget_low_s8(l), vget_low_s8(r));
                let hi = vmull_high_s8(l, r);
                let p = vpaddq_s16(lo, hi);
                vpadalq_s16(acc, p)
            }
        }
    }

    #[target_feature(enable = "neon")]
    pub fn reinterpret_i32(v: I8Vec) -> I32Vec {
        vreinterpretq_s32_s8(v)
    }
}

pub mod i16 {
    use super::*;

    pub const LANES: usize = size_of::<I16Vec>() / size_of::<i16>();

    #[target_feature(enable = "neon")]
    pub unsafe fn load(ptr: *const i16) -> I16Vec {
        unsafe { vld1q_s16(ptr) }
    }

    #[target_feature(enable = "neon")]
    pub unsafe fn store(ptr: *mut i16, v: I16Vec) {
        unsafe { vst1q_s16(ptr, v) }
    }

    #[target_feature(enable = "neon")]
    pub fn splat(n: i16) -> I16Vec {
        vdupq_n_s16(n)
    }

    #[target_feature(enable = "neon")]
    pub fn add(l: I16Vec, r: I16Vec) -> I16Vec {
        vaddq_s16(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn min(l: I16Vec, r: I16Vec) -> I16Vec {
        vminq_s16(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn max(l: I16Vec, r: I16Vec) -> I16Vec {
        vmaxq_s16(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn mulhi_shl7(l: I16Vec, r: I16Vec) -> I16Vec {
        vqdmulhq_s16(l, vshlq_n_s16(r, 6))
    }

    #[target_feature(enable = "neon")]
    pub fn packus(l: I16Vec, r: I16Vec) -> I8Vec {
        vreinterpretq_s8_u8(vqmovun_high_s16(vqmovun_s16(l), r))
    }
}

pub mod i32 {
    use super::*;

    pub const LANES: usize = size_of::<I32Vec>() / size_of::<i32>();

    #[target_feature(enable = "neon")]
    pub unsafe fn load(ptr: *const i32) -> I32Vec {
        unsafe { vld1q_s32(ptr) }
    }

    #[target_feature(enable = "neon")]
    pub unsafe fn store(ptr: *mut i32, v: I32Vec) {
        unsafe { vst1q_s32(ptr, v) }
    }

    #[target_feature(enable = "neon")]
    pub fn splat(n: i32) -> I32Vec {
        vdupq_n_s32(n)
    }

    #[target_feature(enable = "neon")]
    pub fn reinterpret_i8(v: I32Vec) -> I8Vec {
        vreinterpretq_s8_s32(v)
    }

    #[target_feature(enable = "neon")]
    pub fn add(l: I32Vec, r: I32Vec) -> I32Vec {
        vaddq_s32(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn mul(l: I32Vec, r: I32Vec) -> I32Vec {
        vmulq_s32(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn shr_const<const N: i32>(v: I32Vec) -> I32Vec {
        vshrq_n_s32(v, N)
    }

    #[target_feature(enable = "neon")]
    pub fn min(l: I32Vec, r: I32Vec) -> I32Vec {
        vminq_s32(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn max(l: I32Vec, r: I32Vec) -> I32Vec {
        vmaxq_s32(l, r)
    }

    #[target_feature(enable = "neon")]
    pub fn reduce_sum(v: I32Vec) -> i32 {
        vaddvq_s32(v)
    }

    #[target_feature(enable = "neon")]
    pub fn nnz_indices(v: I32Vec) -> (I16Vec, u16) {
        let mask = vtstq_s32(v, v);
        let bitmask = vaddvq_u32(vandq_u32(mask, unsafe { vld1q_u32([1, 2, 4, 8].as_ptr()) }));
        let idxs = unsafe { vld1q_s16(nnz_table::NNZ_TABLE[bitmask as usize].as_ptr()) };
        (idxs, bitmask.count_ones() as u16)
    }
}
