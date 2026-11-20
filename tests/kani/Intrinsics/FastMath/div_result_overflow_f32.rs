// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Regression test for: fdiv_fast with finite inputs producing an infinite result.
// The Rust documentation states that fast math intrinsics have UB if the result
// is not finite (infinite or NaN), not just if the inputs are non-finite.
// Previously, Kani only checked that inputs were finite, missing cases where
// finite inputs produce an infinite result (e.g., f32::MAX / f32::MIN_POSITIVE).

// kani-verify-fail

#![feature(core_intrinsics)]

/// Check that fdiv_fast detects overflow to infinity as UB.
/// f32::MAX / f32::MIN_POSITIVE overflows to infinity, which is UB for fdiv_fast.
#[kani::proof]
fn fdiv_fast_result_overflow_f32() {
    let a = f32::MAX;
    let b = f32::MIN_POSITIVE;
    let _r = unsafe { core::intrinsics::fdiv_fast(a, b) };
}
