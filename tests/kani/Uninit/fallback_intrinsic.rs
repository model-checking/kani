// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! An intrinsic that Kani has no model for, but whose fallback body it uses, is an ordinary call
//! for the memory initialization checks: its fallback body is instrumented like any other
//! function. `Ord::max` and `Ord::min` on integers call the `integer_max` and `integer_min`
//! intrinsics as of nightly-2026-11-17.

#[kani::proof]
fn integer_max_min() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    assert!(a.max(b) >= a && a.max(b) >= b);
    assert!(a.min(b) <= a && a.min(b) <= b);
}

#[kani::proof]
fn max_of_values_behind_pointers() {
    let a: i64 = kani::any();
    let b: i64 = kani::any();
    let (pa, pb) = (&raw const a, &raw const b);
    let m = unsafe { (*pa).max(*pb) };
    assert!(m == a || m == b);
}
