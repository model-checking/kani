// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that `panic!` with args can be used in a const fn

const fn my_const_fn(msg: &str) -> ! {
    panic!("{}", msg)
}

#[kani::proof]
pub fn check_something() {
    let x: u8 = if kani::any() { 3 } else { 95 };
    if x > 100 {
        my_const_fn("x is greater than 100");
    }
}
