// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --default-unwind 5
//
// Check that users can implement Arbitrary to a simple data struct with Vec<>.
extern crate kani;
use kani::{BoundedAny, BoundedArbitrary};

#[derive(BoundedArbitrary)]
#[allow(unused)]
struct MyVector<T> {
    #[bounded]
    vector: Vec<T>,
    cap: usize,
}

/// Check that macro works on enums
#[derive(BoundedArbitrary)]
#[allow(unused)]
enum Enum<T> {
    A(#[bounded] String),
    B(#[bounded] Vec<T>, usize),
    C {
        #[bounded]
        x: Vec<T>,
        y: bool,
    },
}

#[kani::proof]
fn check_my_vec() {
    let my_vec: BoundedAny<MyVector<bool>, 4> = kani::any();
    assert!(my_vec.vector.len() <= 4)
}

#[kani::proof]
fn check_enum() {
    let any_enum: BoundedAny<Enum<bool>, 4> = kani::any();
    match any_enum.into_inner() {
        Enum::A(s) => assert!(s.len() <= 4),
        Enum::B(v, _) => assert!(v.len() <= 4),
        Enum::C { x, y: _ } => assert!(x.len() <= 4),
    }
}
