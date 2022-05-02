// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fsub_fast` triggers overflow checks
// kani-verify-fail

#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();
    let _z = unsafe { std::intrinsics::fsub_fast(x, y) };
}
