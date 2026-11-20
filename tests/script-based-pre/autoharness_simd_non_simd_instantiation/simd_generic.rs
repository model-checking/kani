// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A generic helper whose body calls a SIMD intrinsic, but whose type parameter is not bound to
//! the SIMD types. rustc's codegen backends reject `imin::<i32>` (`E0511`), so no program that
//! `cargo build` accepts contains that call; autoharness synthesizes its own instantiation, though, and
//! used to pick `i32` and crash the SIMD codegen on the vector length of a scalar:
//! <https://github.com/model-checking/kani/issues/4919>. `core_arch::simd::simd_imin` has exactly
//! this shape, which is what blocked autoharness over the standard library.
//!
//! Autoharness now rejects an instantiation that would make the call invalid, so it picks the
//! `#[repr(simd)]` type instead. A caller that makes the invalid instantiation one level down is
//! only caught in codegen, and fails its own harness rather than the run.

#![feature(core_intrinsics, repr_simd)]
#![allow(internal_features)]

use std::intrinsics::simd::{simd_lt, simd_xor};

#[repr(simd)]
#[derive(Copy, Clone)]
pub struct I32x2([i32; 2]);

/// The reproducer: `T` only has to be `Copy`, so `i32` satisfies the bounds but not `simd_lt`.
pub unsafe fn imin<T: Copy>(a: T, b: T) -> T {
    let mask: T = simd_lt(a, b);
    // Kani does not model `simd_select`; combine the lanes with an intrinsic it does model and
    // that cannot overflow, since the point here is which type `T` is instantiated with.
    simd_xor(mask, b)
}

/// Reaches the invalid instantiation through a call, which autoharness does not inspect: its own
/// body calls no SIMD intrinsic, so `i32` is chosen and codegen reports the unsupported construct.
/// Detecting this in the harness selection instead is <https://github.com/model-checking/kani/issues/4926>.
pub unsafe fn calls_imin<T: Copy>(a: T, b: T) -> T {
    imin(a, b)
}

/// A SIMD intrinsic whose operand types are fixed: unaffected by the check.
pub unsafe fn compare_vectors(a: I32x2, b: I32x2) -> I32x2 {
    simd_lt(a, b)
}
