// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that unchecked sub trigger overflow checks.
// rmc-verify-fail

#![feature(unchecked_math)]

pub fn main() {
    let a: u8 = rmc::nondet();
    let b: u8 = rmc::nondet();
    unsafe { a.unchecked_sub(b) };
}
