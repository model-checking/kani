// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// rmc-flags: --unwind 3
// Check that any_raw for arrays do not respect the elements invariants.

extern crate rmc;

use rmc::Invariant;

fn main() {
    let arr_raw: [char; 2] = unsafe { rmc::any_raw() };
    rmc::expect_fail(arr_raw[0].is_valid(), "Should fail");
    rmc::expect_fail(arr_raw[1].is_valid(), "Should fail");
}
