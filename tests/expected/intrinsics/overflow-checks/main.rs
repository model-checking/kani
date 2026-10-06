// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression test for https://github.com/model-checking/kani/issues/4970.
//! Kani compiles with `-Coverflow-checks=on`, so `core::intrinsics::overflow_checks()` must be
//! `true` and library code that branches on it must take its checked path. Kani used to evaluate
//! the intrinsic to `false`: `pow` and the funnel shifts wrapped instead of panicking, and the
//! iterator of `core::range::RangeFrom` panicked before yielding the maximum value.
#![feature(core_intrinsics, funnel_shifts)]

#[kani::proof]
fn check_overflow_checks_intrinsic() {
    assert!(core::intrinsics::overflow_checks());
}

#[kani::proof]
fn check_pow_overflow() {
    let _ = 2u32.pow(32);
    // Unreachable once the call panics, which gives this harness a check of its own.
    kani::cover!(true, "`2u32.pow(32)` returns");
}

#[kani::proof]
fn check_pow_no_overflow() {
    assert_eq!(2u32.pow(31), 1 << 31);
}

#[kani::proof]
fn check_signed_pow_overflow() {
    let _ = 2i32.pow(31);
    kani::cover!(true, "`2i32.pow(31)` returns");
}

#[kani::proof]
fn check_funnel_shl_overflow() {
    let _ = u32::MAX.funnel_shl(u32::MAX, u32::BITS);
}

#[kani::proof]
fn check_range_from_yields_max() {
    let mut iter = core::range::RangeFrom { start: u8::MAX }.into_iter();
    assert_eq!(iter.next(), Some(u8::MAX));
}

#[kani::proof]
fn check_range_from_past_max() {
    let mut iter = core::range::RangeFrom { start: u8::MAX }.into_iter();
    let _ = iter.next();
    let _ = iter.next();
}
