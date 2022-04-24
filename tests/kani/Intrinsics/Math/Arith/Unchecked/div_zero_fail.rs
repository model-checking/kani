// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `unchecked_div` triggers overflow checks.
// Covers the case where `b == 0`.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: i32 = kani::any();
    let b: i32 = 0;
    unsafe { std::intrinsics::unchecked_div(a, b) };
}
