// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that there is a compilation error when the predicate passed to
//! `kani::invariant` attribute would result in a compiler error.

extern crate kani;
use kani::Invariant;

// Note: The `x.is_safe() && y.is_safe()` requires `self` before each struct
// field to be evaluated in the `is_safe` function body.
#[derive(kani::Arbitrary)]
#[kani::invariant(x.is_safe() && y.is_safe())]
struct Point {
    x: i32,
    y: i32,
}
