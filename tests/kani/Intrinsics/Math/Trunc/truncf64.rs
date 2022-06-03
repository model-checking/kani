// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

// Checks that `truncf64` returns the integer part of a number.
#[kani::proof]
fn main() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());

    let trunc_res = unsafe { std::intrinsics::truncf64(x) };

    // The expected result is the number minus its fractional part
    let expected_res = x - x.fract();
    assert!(trunc_res == expected_res);
}
