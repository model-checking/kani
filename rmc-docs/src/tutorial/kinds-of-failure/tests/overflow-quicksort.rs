// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

// ANCHOR: code
fn find_midpoint(low: u32, high: u32) -> u32 {
    return (low + high) / 2;
}
// ANCHOR_END: code

// ANCHOR: rmc
#[cfg(rmc)]
#[no_mangle]
fn main() {
    let a: u32 = rmc::any();
    let b: u32 = rmc::any();
    find_midpoint(a, b);
}
// ANCHOR_END: rmc
