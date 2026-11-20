// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A generic helper whose body stores a SIMD comparison's result in `T` itself, as
//! `core_arch::simd::simd_imax<T: Copy>` does. The result of a comparison is a mask, which needs
//! integer lanes, so instantiating `T` with a float vector (`core_arch`'s `__m128`) is invalid.
//! Kani used to report that as a compile error, which aborted autoharness over the whole crate:
//! <https://github.com/model-checking/kani/issues/4950>.

#![feature(core_intrinsics, repr_simd)]
#![allow(internal_features, non_camel_case_types)]

use std::intrinsics::simd::{simd_gt, simd_xor};

#[repr(simd)]
#[derive(Clone, Copy)]
pub struct f32x4([f32; 4]);

#[repr(simd)]
#[derive(Clone, Copy)]
pub struct i32x4([i32; 4]);

#[repr(simd)]
#[derive(Clone, Copy)]
pub struct i64x2([i64; 2]);

#[cfg(kani)]
impl kani::Arbitrary for f32x4 {
    fn any() -> Self {
        f32x4(kani::any())
    }
}

#[cfg(kani)]
impl kani::Arbitrary for i32x4 {
    fn any() -> Self {
        i32x4(kani::any())
    }
}

#[cfg(kani)]
impl kani::Arbitrary for i64x2 {
    fn any() -> Self {
        i64x2(kani::any())
    }
}

/// Only a float vector satisfies this bound.
pub trait FloatLanes: Copy {}
impl FloatLanes for f32x4 {}

/// Both a float and an integer vector satisfy this one.
pub trait AnyLanes: Copy {}
impl AnyLanes for f32x4 {}
impl AnyLanes for i32x4 {}

/// An operand with four integer lanes.
pub trait FourLanes: Copy {}
impl FourLanes for i32x4 {}

/// Only a scalar satisfies this, so a comparison result bounded by it is never a vector.
pub trait Scalar: Copy {}
impl Scalar for i32 {}

/// Only a two-lane integer vector satisfies this.
pub trait TwoLanes: Copy {}
impl TwoLanes for i64x2 {}

// Kani does not model `simd_select`; `simd_xor` combines the lanes instead, since the point here is
// which type `T` is instantiated with.

/// Only an invalid instantiation exists, so autoharness skips this and says why.
pub unsafe fn float_only<T: FloatLanes>(a: T, b: T) -> T {
    let mask: T = simd_gt(a, b);
    simd_xor(mask, b)
}

/// Autoharness passes over the float vector and picks the integer one.
pub unsafe fn int_available<T: AnyLanes>(a: T, b: T) -> T {
    let mask: T = simd_gt(a, b);
    simd_xor(mask, b)
}

/// The result is a scalar, not a mask. Codegen used to unwrap the scalar's vector length here.
pub unsafe fn scalar_result<T: FourLanes, M: Scalar>(a: T, b: T) -> M {
    simd_gt(a, b)
}

/// The result is an integer mask, but with two lanes for a four-lane comparison.
pub unsafe fn wrong_lane_count<T: FourLanes, M: TwoLanes>(a: T, b: T) -> M {
    simd_gt(a, b)
}

/// The invalid comparison is one level down, which autoharness does not inspect
/// (<https://github.com/model-checking/kani/issues/4926>): it picks the float vector, and the
/// comparison fails this harness instead of the run.
pub unsafe fn calls_float_only<T: FloatLanes>(a: T, b: T) -> T {
    float_only(a, b)
}
