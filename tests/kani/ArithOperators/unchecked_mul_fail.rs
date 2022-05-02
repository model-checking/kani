// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that unchecked mul trigger overflow checks.
// kani-verify-fail

#![feature(unchecked_math)]

#[kani::proof]
fn main() {
    let a: u8 = kani::nondet();
    let b: u8 = kani::nondet();
    unsafe { a.unchecked_mul(b) };
}
