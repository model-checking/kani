// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that any_raw for arrays do not respect the elements invariants.

extern crate kani;

use kani::Invariant;

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let arr_raw: [char; 2] = unsafe { kani::any_raw() };
    kani::expect_fail(arr_raw[0].is_valid(), "Should fail");
    kani::expect_fail(arr_raw[1].is_valid(), "Should fail");
}
