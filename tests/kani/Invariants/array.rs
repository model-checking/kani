// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for array respect the underlying types invariant.

extern crate kani;

use kani::Invariant;

#[kani::proof]
#[kani::unwind(4)]
fn main() {
    let arr: [bool; 2] = kani::any();
    assert!((0..=1).contains(&(arr[0] as u8)));
    assert!((0..=1).contains(&(arr[1] as u8)));
}
