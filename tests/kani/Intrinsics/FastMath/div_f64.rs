// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fdiv_fast` overflow checks pass with suitable assumptions

#![feature(core_intrinsics)]

// Unconstrained floating point values will cause performance issues in this
// operation. To avoid them, we assume values within a moderate range.
const MIN_FP_VALUE: f64 = 0.1;
const MAX_FP_VALUE: f64 = 100.0;

fn assume_fp_range(val: f64) {
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
    let x: f64 = kani::any();
    let y: f64 = kani::any();

    kani::assume(x.is_finite());
    kani::assume(y.is_finite());
    assume_fp_range(x);
    assume_fp_range(y);

    let _z = unsafe { std::intrinsics::fdiv_fast(x, y) };
}
