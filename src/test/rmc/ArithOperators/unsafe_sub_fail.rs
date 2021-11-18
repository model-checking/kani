// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that regular arithmetic operations in unsafe blocks still trigger overflow checks.
// rmc-verify-fail
// rmc-flags: --function check_sub

pub fn check_sub(a: u8, b: u8) {
    unsafe {
        a - b;
    }
}
