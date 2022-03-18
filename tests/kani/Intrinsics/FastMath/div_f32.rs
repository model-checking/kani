// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fdiv_fast` overflow checks pass with suitable assumptions

#![feature(core_intrinsics)]

// Unconstrained floating point values will cause performance issues in this
// operation. To avoid them, we assume values within a moderate range.
const MIN_FP_VALUE: f32 = 0.1;
const MAX_FP_VALUE: f32 = 100.0;

fn assume_fp_range(val: f32) {
    if val.is_sign_positive() {
        kani::assume(val > MIN_FP_VALUE);
        kani::assume(val < MAX_FP_VALUE);
    } else {
        kani::assume(val < -MIN_FP_VALUE);
        kani::assume(val > -MAX_FP_VALUE);
    }
}

#[kani::proof]
fn main() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();

    kani::assume(x.is_finite());
    kani::assume(y.is_finite());
    assume_fp_range(x);
    assume_fp_range(y);

    // Comparing `z` and `w` below causes serious performance issues
    // https://github.com/model-checking/kani/issues/809
    let z = unsafe { std::intrinsics::fdiv_fast(x, y) };
    // let w = x / y;
    // assert!(z == w);
}
