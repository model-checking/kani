// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This test verifies that Kani does not report floating-point overflow by default
// for operations that result in +/-Infinity.
extern crate kani;

// Use the result so rustc doesn't optimize them away.
fn dummy(result: f32) -> f32 {
    result
}

#[kani::proof]
fn main() {
    let a = kani::any_where(|&x: &f32| x.is_finite());
    let b = kani::any_where(|&x: &f32| x.is_finite());

    dummy(a + b);
    dummy(a - b);
    dummy(a * b);
    dummy(-a);
}
