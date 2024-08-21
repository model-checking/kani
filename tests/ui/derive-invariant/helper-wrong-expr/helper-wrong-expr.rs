// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check the compilation error for the `#[safety_constraint(...)]` attribute helper when the
//! argument cannot be evaluated in the struct's context.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    // Note: `x` is a reference in this context, we should refer to `*x`
    #[safety_constraint(x >= 0)]
    x: i32,
    #[safety_constraint(*y >= 0)]
    y: i32,
}
