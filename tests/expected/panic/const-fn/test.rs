// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that `panic!` with args can be used in a const fn

const fn my_const_fn(msg: &str) -> ! {
    panic!("{}", msg)
}

#[kani::proof]
pub fn check_something() {
    let x = 5;
    if x > 2 {
        my_const_fn("function will panic");
    }
}
