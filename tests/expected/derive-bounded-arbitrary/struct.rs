// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that derive BoundedArbitrary macro works on enums

extern crate kani;
use kani::BoundedArbitrary;

#[derive(BoundedArbitrary)]
#[allow(unused)]
struct MyVector<T> {
    #[bounded]
    vector: Vec<T>,
    cap: usize,
}

#[kani::proof]
#[kani::unwind(6)]
fn check_my_vec() {
    let my_vec: MyVector<bool> = kani::bounded_any::<_, 4>();
    for i in 0..=4 {
        kani::cover!(my_vec.vector.len() == i);
    }
}
