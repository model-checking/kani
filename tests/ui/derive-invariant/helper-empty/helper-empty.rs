// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check the compilation error for the invariant attribute helper when an
//! argument isn't provided.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[safety_constraint]
    x: i32,
    #[safety_constraint(*y >= 0)]
    y: i32,
}
