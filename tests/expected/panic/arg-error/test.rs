// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021

//! This test checks that Kani processes arguments of panic macros and produces
//! a compile error for invalid arguments (e.g. missing argument)

fn my_const_fn(msg: &str) -> ! {
    core::panic!("{}")
}

#[kani::proof]
fn check_panic_arg_error() {
    my_const_fn("failed");
}
