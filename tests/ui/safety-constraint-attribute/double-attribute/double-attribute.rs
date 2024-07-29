// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that there is a compilation error when the predicate passed to
//! `kani::invariant` attribute would result in a compiler error.

extern crate kani;
use kani::Invariant;

// Note: The struct fields `x` and `y` are references in this context, we should
// refer to `*x` and `*y` instead.
#[derive(Invariant)]
#[safety_constraint(*x >= 0)]
#[safety_constraint(*y >= 0)]
struct Point {
    x: i32,
    y: i32,
}
