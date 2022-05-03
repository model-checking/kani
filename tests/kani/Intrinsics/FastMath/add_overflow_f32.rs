// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fadd_fast` triggers overflow checks
// kani-verify-fail

#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    let _z = unsafe { std::intrinsics::fadd_fast(x, y) };
}
