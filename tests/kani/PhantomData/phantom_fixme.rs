// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// compile-flags: --edition 2018
// kani-flags: --default-unwind 4
//
// Testcase based on https://doc.rust-lang.org/rust-by-example/generics/phantom/testcase_units.html
// which reproduces issue https://github.com/model-checking/kani/issues/560

use std::marker::PhantomData;
use std::ops::Add;

#[derive(Debug, Clone, Copy)]
enum Mm {}

/// `Length` is a type with phantom type parameter `Unit`,
/// and is not generic over the length type (that is `u32`).
///
/// `u32` already implements the `Clone` and `Copy` traits.
#[derive(Debug, Clone, Copy)]
struct Length<Unit>(u32, PhantomData<Unit>);

/// The `Add` trait defines the behavior of the `+` operator.
impl<Unit> Add for Length<Unit> {
    type Output = Length<Unit>;

    // add() returns a new `Length` struct containing the sum.
    fn add(self, rhs: Length<Unit>) -> Length<Unit> {
        // `+` calls the `Add` implementation for `u32`.
        Length(self.0 + rhs.0, PhantomData)
    }
}

#[kani::proof]
fn main() {
    // `one_meter` has phantom type parameter `Mm`.
    let one_meter: Length<Mm> = Length(1000, PhantomData);

    // `+` calls the `add()` method we implemented for `Length<Unit>`.
    //
    // Since `Length` implements `Copy`, `add()` does not consume
    // `one_foot` and `one_meter` but copies them into `self` and `rhs`.
    let two_meters = one_meter + one_meter;

    assert!(two_meters.0 == 2000);
}
