// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for array respect the underlying types invariant.

extern crate kani;

use kani::Invariant;

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let arr: [char; 2] = kani::any();
    assert!(arr[0].is_valid());
    assert!(arr[1].is_valid());
}
