#![allow(clippy::needless_range_loop, clippy::missing_transmute_annotations)]

use std::mem::{MaybeUninit, transmute};

use crate::nnue::{
    network::{L1, L2, L3, NET},
    simd,
};

const Q0: i16 = 255;
const _Q1: i16 = 128;
const Q: i32 = 64;
const SCALE: i32 = 400;

#[inline(always)]
fn activate_ft(us: &[i16; L1], them: &[i16; L1]) -> [i8; L1] {
    let mut out = [const { MaybeUninit::<i8>::uninit() }; L1];

    const { assert!((L1 / 2).is_multiple_of(2 * simd::i16::LANES)) };

    unsafe {
        for i in (0..L1 / 2).step_by(2 * simd::i16::LANES) {
            use simd::i16::{LANES, load, max, min, mulhi_shl7, packus, splat};

            let mut us1 = load(us.as_ptr().add(i));
            let mut us2 = load(us.as_ptr().add(i + L1 / 2));
            let mut us3 = load(us.as_ptr().add(i + LANES));
            let mut us4 = load(us.as_ptr().add(i + L1 / 2 + LANES));

            let mut them1 = load(them.as_ptr().add(i));
            let mut them2 = load(them.as_ptr().add(i + L1 / 2));
            let mut them3 = load(them.as_ptr().add(i + LANES));
            let mut them4 = load(them.as_ptr().add(i + L1 / 2 + LANES));

            // We can save the max(_, 0) on some of the vectors, as `packus` will clamp any negative values to 0.
            us1 = min(us1, splat(Q0));
            us3 = min(us3, splat(Q0));
            us2 = min(max(us2, splat(0)), splat(Q0));
            us4 = min(max(us4, splat(0)), splat(Q0));

            them1 = min(them1, splat(Q0));
            them3 = min(them3, splat(Q0));
            them2 = min(max(them2, splat(0)), splat(Q0));
            them4 = min(max(them4, splat(0)), splat(Q0));

            let us_pair1 = mulhi_shl7(us1, us2);
            let us_pair2 = mulhi_shl7(us3, us4);

            let them_pair1 = mulhi_shl7(them1, them2);
            let them_pair2 = mulhi_shl7(them3, them4);

            let p1 = packus(us_pair1, us_pair2);
            let p2 = packus(them_pair1, them_pair2);

            simd::i8::store(out.as_mut_ptr().add(i).cast(), p1);
            simd::i8::store(out.as_mut_ptr().add(i + L1 / 2).cast(), p2);
        }

        MaybeUninit::assume_init(out.into())
    }
}

#[inline(always)]
fn propagate_l1(act_ft: &[i8; L1]) -> [i32; L2] {
    const UNROLL: usize = 4;
    const { assert!(L2.is_multiple_of(simd::i32::LANES)) };
    const { assert!((L1 / 4).is_multiple_of(UNROLL)) };

    unsafe {
        let mut nnz_idxs = [const { MaybeUninit::<i16>::uninit() }; L1 / 4 + simd::i16::LANES];
        let mut nnz_count = 0;
        {
            let mut base = simd::i16::splat(0);
            for i in (0..L1).step_by(simd::i8::LANES) {
                let chunk = simd::i8::reinterpret_i32(simd::i8::load(act_ft.as_ptr().add(i)));
                let (idxs, cnt) = simd::i32::nnz_indices(chunk);
                simd::i16::store(
                    nnz_idxs.as_mut_ptr().add(nnz_count).cast(),
                    simd::i16::add(base, idxs),
                );
                nnz_count += cnt as usize;
                base = simd::i16::add(base, simd::i16::splat(simd::i32::LANES as i16));
            }
        }

        // in [0, Q0^2 * Q1 / 2^9]
        let mut intermediate = [[simd::i32::splat(0); UNROLL]; { L2 / simd::i32::LANES }];
        let ft_32 = act_ft.as_chunks::<4>().0;

        let mut i_outer = 0;

        while i_outer + UNROLL <= nnz_count {
            for j in 0..(L2 / simd::i32::LANES) {
                for i_inner in 0..UNROLL {
                    let i = nnz_idxs[i_outer + i_inner].assume_init() as usize;
                    let ft_vec = simd::i32::splat(transmute(ft_32[i]));
                    intermediate[j][i_inner] = simd::i8::dpbusd(
                        intermediate[j][i_inner],
                        simd::i32::reinterpret_i8(ft_vec),
                        simd::i8::load(NET.l1w[i].as_ptr().add(j * simd::i8::LANES)),
                    );
                }
            }

            i_outer += UNROLL;
        }

        while i_outer < nnz_count {
            let i = nnz_idxs[i_outer].assume_init() as usize;
            let ft_vec = simd::i32::splat(transmute(ft_32[i]));
            for j in 0..(L2 / simd::i32::LANES) {
                intermediate[j][0] = simd::i8::dpbusd(
                    intermediate[j][0],
                    simd::i32::reinterpret_i8(ft_vec),
                    simd::i8::load(NET.l1w[i].as_ptr().add(j * simd::i8::LANES)),
                )
            }
            i_outer += 1;
        }

        let mut out = [0; L2];
        for i in 0..L2 / simd::i32::LANES {
            use simd::i32::*;

            let bias = load(NET.l1b.as_ptr().add(i * LANES));

            let mut sum = splat(0);
            for partial_sum in intermediate[i] {
                sum = add(sum, partial_sum);
            }

            let shifted = add(bias, shr_const::<8>(sum));
            let clamped = min(max(shifted, splat(0)), splat(Q));
            let activated = mul(clamped, clamped);
            store(out.as_mut_ptr().add(i * LANES), activated);
        }
        out
    }
}

fn propagate_l2(act_l1: &[i32; L2]) -> [i32; L3] {
    use simd::{I32Vec, i32::*};
    unsafe {
        let mut sums: [I32Vec; L3 / LANES] =
            std::array::from_fn(|i| load(NET.l2b.as_ptr().add(i * LANES)));

        for i in 0..L2 {
            let r = splat(act_l1[i]);
            for j in 0..L3 / LANES {
                let l = load(NET.l2w[i].as_ptr().add(j * LANES));
                sums[j] = add(sums[j], mul(l, r));
            }
        }

        let mut out = [0i32; L3];
        for (i, sum) in sums.iter().enumerate() {
            let clamped = min(max(*sum, splat(0)), splat(Q.pow(3)));
            store(out.as_mut_ptr().add(i * LANES), clamped);
        }

        out
    }
}

fn propagate_l3(act_l2: &[i32; L3]) -> i32 {
    use simd::i32::*;
    unsafe {
        let mut sum = splat(0);
        for i in (0..L3).step_by(LANES) {
            let l = load(act_l2.as_ptr().add(i));
            let r = load(NET.l3w.as_ptr().add(i));
            sum = add(sum, mul(l, r));
        }

        reduce_sum(sum) + NET.l3b
    }
}

#[inline(never)]
pub fn forward(us: &[i16; L1], them: &[i16; L1]) -> i32 {
    // in [0, Q1]
    let act_ft = activate_ft(us, them);
    // in [0, Q^2]
    let act_l1 = propagate_l1(&act_ft);
    // in [0, Q^3]
    let act_l2 = propagate_l2(&act_l1);
    // in [0, SCALE * Q^4]
    let scaled = propagate_l3(&act_l2) as i64 * SCALE as i64;

    (scaled / (Q.pow(4) as i64)) as i32
}
