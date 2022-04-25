// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fsub_fast` overflow checks pass with suitable assumptions

#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();

    kani::assume(x.is_finite());
    kani::assume(y.is_finite());
    match (x.is_sign_positive(), y.is_sign_positive()) {
        (true, false) => kani::assume(x < f32::MAX + y),
        (false, true) => kani::assume(x > f32::MIN + y),
        _ => (),
    }
    let z = unsafe { std::intrinsics::fsub_fast(x, y) };
    let w = x - y;
    assert!(z == w);
}
