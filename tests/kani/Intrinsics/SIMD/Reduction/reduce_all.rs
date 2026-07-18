// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that the `simd_reduce_all` intrinsic (a boolean AND-reduction over a
//! mask-like integer vector, whose lanes must be `0` or `!0`) is supported and
//! returns the expected results. This is also a regression test for the
//! portable-SIMD mask validation (`Mask::valid`, used by `Mask::from_array` and
//! friends) and `Mask::all`, both of which lower through this intrinsic.
#![feature(repr_simd, core_intrinsics, portable_simd)]
use std::intrinsics::simd::simd_reduce_all;
use std::simd::{Mask, Simd};

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct i8x4([i8; 4]);

#[kani::proof]
fn check_reduce_all_true() {
    let mask = i8x4([!0, !0, !0, !0]);
    assert!(unsafe { simd_reduce_all(mask) });
}

#[kani::proof]
fn check_reduce_all_one_false() {
    let mask = i8x4([!0, !0, 0, !0]);
    assert!(!unsafe { simd_reduce_all(mask) });
}

#[kani::proof]
fn check_reduce_all_symbolic() {
    let lanes: [bool; 4] = kani::any();
    let mask = i8x4(std::array::from_fn(|i| if lanes[i] { !0 } else { 0 }));
    let expected = lanes.iter().all(|&b| b);
    assert_eq!(unsafe { simd_reduce_all(mask) }, expected);
}

#[kani::proof]
fn check_mask_all() {
    let vals: [bool; 4] = kani::any();
    let repr: Simd<i8, 4> = Simd::from_array(std::array::from_fn(|i| if vals[i] { !0 } else { 0 }));
    // SAFETY: every lane of `repr` is 0 or !0.
    // (`Mask::from_array` is not used here because it also lowers through
    // `simd_cast`, which Kani does not support yet.)
    let mask = unsafe { Mask::<i8, 4>::from_simd_unchecked(repr) };
    assert_eq!(mask.all(), vals.iter().all(|&b| b));
}
