// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani does not emit "unused variable" warnings for
//! variables that are only used in arguments of panic/unreachable macros
//! This doesn't work currently:
//! https://github.com/model-checking/kani/issues/1556

// Promote "unused variable" warnings to an error so that this test fails if
// Kani's overridden version of the panic/unreachable macros drops variables
// used as arguments of those macros
#![deny(unused_variables)]

#[kani::proof]
fn arg_in_macro() {
    let x: Option<i32> = None;
    match x {
        Some(y) => panic!("Value of y is {}", y),
        None => {}
    }
    match x {
        Some(y) => unreachable!("Value of y is {}", y),
        None => {}
    }
}
