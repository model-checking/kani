// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check we don't print temporary variables as part of CBMC messages.
extern crate kani;

use kani::any;

// Ensure rustc encodes the operation.
fn dummy(var: i32) {
    kani::assume(var != 0);
}

#[kani::proof]
fn main() {
    match kani::any() {
        0 => dummy(any::<i32>() + any::<i32>()),
        1 => dummy(any::<i32>() - any::<i32>()),
        2 => dummy(any::<i32>() * any::<i32>()),
        3 => dummy(any::<i32>() / any::<i32>()),
        4 => dummy(any::<i32>() % any::<i32>()),
        5 => dummy(any::<i32>() << any::<i32>()),
        6 => dummy(any::<i32>() >> any::<i32>()),
        _ => (),
    }
}
