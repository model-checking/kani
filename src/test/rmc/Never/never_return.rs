// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

#![feature(never_type)]

/// Test using the never type
pub fn err() -> ! {
    panic!("EXPECTED FAIL: Function should always fail");
}

// Give an empty main to make rustc happy.
#[no_mangle]
fn main() {
    let var = rmc::nondet::<i32>();
    if var > 0 {
        err();
    }
}
