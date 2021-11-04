// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(never_type)]

/// Test using the never type
pub fn err() -> ! {
    panic!("EXPECTED FAIL: Function should always fail");
}

// Give an empty main to make rustc happy.
#[no_mangle]
pub fn main() {
    //let var = rmc::nondet::<i32>();
    let var = 2;
    if var > 0 {
        err();
    }
}
