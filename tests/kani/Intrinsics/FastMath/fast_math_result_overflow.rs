// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Regression test for: fadd_fast with finite inputs producing infinite result.
// The Rust documentation states that fast math intrinsics have UB if the result
// is not finite (infinite or NaN), not just if the inputs are non-finite.
// Previously, Kani only checked that inputs were finite, missing cases where
// finite inputs produce an infinite result (e.g., f32::MAX + f32::MAX).

// kani-verify-fail

#![feature(core_intrinsics)]

/// Check that fadd_fast detects overflow to infinity as UB.
/// f32::MAX + f32::MAX overflows to infinity, which is UB for fadd_fast.
#[kani::proof]
fn fadd_fast_result_overflow_f32() {
    let a = f32::MAX;
    let _r = unsafe { core::intrinsics::fadd_fast(a, a) };
}

/// Check that fmul_fast detects overflow to infinity as UB.
/// f32::MAX * 2.0 overflows to infinity, which is UB for fmul_fast.
#[kani::proof]
fn fmul_fast_result_overflow_f32() {
    let a = f32::MAX;
    let b = 2.0f32;
    let _r = unsafe { core::intrinsics::fmul_fast(a, b) };
}

/// Check that fsub_fast detects underflow to negative infinity as UB.
/// (-f32::MAX) - f32::MAX underflows to negative infinity.
#[kani::proof]
fn fsub_fast_result_overflow_f32() {
    let a = -f32::MAX;
    let b = f32::MAX;
    let _r = unsafe { core::intrinsics::fsub_fast(a, b) };
}

/// Check that fdiv_fast detects overflow as UB.
/// f32::MAX / f32::MIN_POSITIVE produces infinity.
#[kani::proof]
fn fdiv_fast_result_overflow_f32() {
    let a = f32::MAX;
    let b = f32::MIN_POSITIVE;
    let _r = unsafe { core::intrinsics::fdiv_fast(a, b) };
}
