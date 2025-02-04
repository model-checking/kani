// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check we don't print temporary variables as part of CBMC messages.
extern crate kani;

use kani::any;

// Use the result so rustc doesn't optimize them away.
fn dummy(result: f32) -> f32 {
    result
}

#[kani::proof]
fn main() {
    let a = kani::any_where(|&x: &f32| x.is_finite() && x.abs() < 1e20);
    let b = kani::any_where(|&x: &f32| x.is_finite() && x.abs() < 1e20);

    dummy(a + b);
    dummy(a - b);
    dummy(a * b);
    dummy(-a);
}
