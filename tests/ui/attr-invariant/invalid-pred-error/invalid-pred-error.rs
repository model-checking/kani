// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Invariant` for structs with named fields.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[kani::invariant(x.is_safe() && y.is_safe())]
struct Point {
    x: i32,
    y: i32,
}
