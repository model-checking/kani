// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Invariant` for structs with generics.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct Point<X, Y> {
    x: X,
    y: Y,
}

#[kani::proof]
fn check_generic_struct_invariant() {
    let point: Point<i32, i8> = kani::any();
    assert!(point.is_safe());
}
