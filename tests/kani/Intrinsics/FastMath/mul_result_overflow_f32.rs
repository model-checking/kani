// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Regression test for: fmul_fast with finite inputs producing an infinite result.
// The Rust documentation states that fast math intrinsics have UB if the result
// is not finite (infinite or NaN), not just if the inputs are non-finite.
// Previously, Kani only checked that inputs were finite, missing cases where
// finite inputs produce an infinite result (e.g., f32::MAX * 2.0).

// kani-verify-fail

#![feature(core_intrinsics)]

/// Check that fmul_fast detects overflow to infinity as UB.
/// f32::MAX * 2.0 overflows to infinity, which is UB for fmul_fast.
#[kani::proof]
fn fmul_fast_result_overflow_f32() {
    let a = f32::MAX;
    let b = 2.0f32;
    let _r = unsafe { core::intrinsics::fmul_fast(a, b) };
}
