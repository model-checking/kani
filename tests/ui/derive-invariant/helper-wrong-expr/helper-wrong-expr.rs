// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check the compilation error for the invariant attribute helper when the
//! argument cannot be evaluated in the struct's context.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    // Note: `x` is unknown, we should refer to `*x`
    #[invariant(x >= 0)]
    x: i32,
    #[invariant(*y >= 0)]
    y: i32,
}
