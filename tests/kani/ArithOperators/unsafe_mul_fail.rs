// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that regular arithmetic operations in unsafe blocks still trigger overflow checks.
// kani-verify-fail
// kani-flags: --function check_mul
// compile-flags: --crate-type lib

#[kani::proof]
pub fn check_mul(a: u8, b: u8) {
    unsafe {
        a * b;
    }
}
