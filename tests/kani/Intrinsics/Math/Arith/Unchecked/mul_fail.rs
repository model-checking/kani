// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `unchecked_mul` triggers overflow checks.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: i32 = kani::any();
    let b: i32 = kani::any();
    // Black box this so it doesn't get pruned by the compiler.
    std::hint::black_box(unsafe { std::intrinsics::unchecked_mul(a, b) });
}
