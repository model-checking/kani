// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `unchecked_shr` triggers overflow checks.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    // Black box this so it doesn't get pruned by the compiler.
    std::hint::black_box(unsafe { std::intrinsics::unchecked_shr(a, b) });
}
