// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `unchecked_shl` triggers overflow checks.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    unsafe { std::intrinsics::unchecked_shl(a, b) };
}
