// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that storing the result of a vector comparison in a scalar is reported
//! as an unsupported construct. rustc's codegen backends reject it with E0511;
//! Kani used to crash reading the scalar's lane count, and now reports it per
//! harness rather than aborting the crate (see #4950).
#![feature(repr_simd, core_intrinsics)]
use std::intrinsics::simd::simd_eq;

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct u64x2([u64; 2]);

#[kani::proof]
fn main() {
    let x = u64x2([0, 0]);
    let y = u64x2([0, 1]);

    unsafe {
        let invalid_simd: u64 = simd_eq(x, y);
        assert!(invalid_simd == 0);
        // ^^^^ The code above fails to type-check in Rust with the error:
        // ```
        // error[E0511]: invalid monomorphization of `simd_eq` intrinsic: expected SIMD return type, found non-SIMD `u64`
        // ```
    }
}
