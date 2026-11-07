// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that a SIMD intrinsic instantiated with a scalar type is reported as unsupported instead
//! of crashing the compiler. rustc rejects such an instantiation in its LLVM backend (E0511), which
//! Kani replaces; autoharness reaches it by instantiating stdarch's generic `simd_imin::<T>` with
//! `T = i32`.
#![feature(core_intrinsics)]
#![allow(internal_features)]
use std::intrinsics::simd::{simd_lt, simd_reduce_max, simd_select, simd_splat};

/// The shape of stdarch's `simd_imin`: `simd_lt` takes the vector as its first argument.
unsafe fn imin<T: Copy>(a: T, b: T) -> T {
    let mask: T = unsafe { simd_lt(a, b) };
    unsafe { simd_select(mask, a, b) }
}

/// `simd_splat` returns the vector instead.
unsafe fn splat<T, U>(value: U) -> T {
    unsafe { simd_splat(value) }
}

/// `simd_reduce_max` takes the vector as its only argument and returns a lane.
unsafe fn reduce_max<T, U>(x: T) -> U {
    unsafe { simd_reduce_max(x) }
}

#[kani::proof]
fn check_scalar_first_argument() {
    let _ = unsafe { imin::<i32>(kani::any(), kani::any()) };
}

#[kani::proof]
fn check_scalar_return_type() {
    let _: i32 = unsafe { splat::<i32, i32>(kani::any()) };
}

#[kani::proof]
fn check_scalar_reduction() {
    let _: i32 = unsafe { reduce_max::<i32, i32>(kani::any()) };
}
