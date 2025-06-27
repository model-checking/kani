// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that there is a compilation error when the `#[safety_constraint(...)]`
//! attribute for struct and the `#[safety_constraint(...)]` attribute for
//! fields is used at the same time.

extern crate kani;
use kani::Invariant;

#[derive(Invariant)]
#[safety_constraint(*x >= 0)]
struct Point {
    x: i32,
    #[safety_constraint(*y >= 0)]
    y: i32,
}
