// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that regular arithmetic operations in unsafe blocks still trigger overflow checks.
// kani-verify-fail

#[kani::proof]
pub fn check_mul(a: u8, b: u8) {
    unsafe {
        a * b;
    }
}
