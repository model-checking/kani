// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that derive BoundedArbitrary macro works on structs with a generic default
//! which had an issue in the past:
//! https://github.com/model-checking/kani/issues/4116

extern crate kani;
use kani::BoundedArbitrary;

#[derive(BoundedArbitrary)]
#[allow(unused)]
struct MyVector<T = i32> {
    #[bounded]
    vector: Vec<T>,
}

#[kani::proof]
#[kani::unwind(6)]
fn check_my_vec() {
    let my_vec: MyVector<u8> = kani::bounded_any::<_, 1>();
    kani::cover!(my_vec.vector.len() == 0);
    kani::cover!(my_vec.vector.len() == 1);
}
