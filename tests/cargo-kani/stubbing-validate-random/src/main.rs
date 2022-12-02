// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests stubbing `rand::random` with a function that has a (harmless)
//! trait mismatch.

trait MyTrait {
    fn get() -> Self;
}

impl MyTrait for i32 {
    fn get() -> Self {
        42
    }
}

fn not_so_random<T: MyTrait>() -> T {
    T::get()
}

#[kani::proof]
#[kani::stub(rand::random, not_so_random)]
fn main() {
    assert_eq!(rand::random::<i32>(), 42);
}
