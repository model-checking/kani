// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that there is a compilation error when the predicate passed to the
//! `#[safety_constraint(...)]` attribute would result in a compiler error.
//!
//! Note: the `#[derive(kani::Invariant)]` macro is required for the compiler error,
//! otherwise the `#[safety_constraint(...)]` attribute is ignored.

extern crate kani;

// Note: The struct fields `x` and `y` are references in this context, we should
// refer to `*x` and `*y` instead.
#[derive(kani::Invariant)]
#[safety_constraint(x >= 0 && y >= 0)]
struct Point {
    x: i32,
    y: i32,
}
