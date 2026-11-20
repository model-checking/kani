// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Regression test for: fsub_fast with finite inputs producing an infinite result.
// The Rust documentation states that fast math intrinsics have UB if the result
// is not finite (infinite or NaN), not just if the inputs are non-finite.
// Previously, Kani only checked that inputs were finite, missing cases where
// finite inputs produce an infinite result (e.g., (-f32::MAX) - f32::MAX).

// kani-verify-fail

#![feature(core_intrinsics)]

/// Check that fsub_fast detects overflow to negative infinity as UB.
/// (-f32::MAX) - f32::MAX overflows to negative infinity, which is UB for fsub_fast.
#[kani::proof]
fn fsub_fast_result_overflow_f32() {
    let a = -f32::MAX;
    let b = f32::MAX;
    let _r = unsafe { core::intrinsics::fsub_fast(a, b) };
}
