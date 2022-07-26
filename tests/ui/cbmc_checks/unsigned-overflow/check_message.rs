// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check we don't print temporary variables as part of CBMC messages.
// cbmc-flags: --unsigned-overflow-check
extern crate kani;

use kani::any;

// Ensure rustc encodes the operation.
fn dummy(var: u32) {
    kani::assume(var != 0);
}

#[kani::proof]
fn main() {
    match kani::any() {
        0 => dummy(any::<u32>() + any::<u32>()),
        1 => dummy(any::<u32>() - any::<u32>()),
        2 => dummy(any::<u32>() * any::<u32>()),
        3 => dummy(any::<u32>() / any::<u32>()),
        4 => dummy(any::<u32>() % any::<u32>()),
        5 => dummy(any::<u32>() << any::<u32>()),
        6 => dummy(any::<u32>() >> any::<u32>()),
        _ => ()
    }
}

