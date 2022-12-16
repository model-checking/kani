// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that the panic macro does not cause a "recursion limit
//! reached" compiler error if the user crate contains an `extern crate std as
//! core` line (see https://github.com/model-checking/kani/issues/1949)

extern crate std as core;

#[kani::proof]
fn main() {
    let x = if kani::any() { 11 } else { 33 };
    if x < 10 {
        panic!("x is {}", x);
    }
}
