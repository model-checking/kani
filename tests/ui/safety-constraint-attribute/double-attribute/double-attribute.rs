// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that there is a compilation error when the `#[safety_constraint(...)]`
//! attribute is used more than once on the same struct.

extern crate kani;
use kani::Invariant;

#[derive(Invariant)]
#[safety_constraint(*x >= 0)]
#[safety_constraint(*y >= 0)]
struct Point {
    x: i32,
    y: i32,
}
