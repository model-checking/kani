// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fsub_fast` overflow checks pass with suitable assumptions

#![feature(core_intrinsics)]

fn main() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();

    kani::assume(x.is_finite());
    kani::assume(y.is_finite());
    match (x.is_sign_positive(), y.is_sign_positive()) {
        (true, false) => kani::assume(x < f64::MAX + y),
        (false, true) => kani::assume(x > f64::MIN + y),
        _ => (),
    }
    let _z = unsafe { std::intrinsics::fsub_fast(x, y) };
}
