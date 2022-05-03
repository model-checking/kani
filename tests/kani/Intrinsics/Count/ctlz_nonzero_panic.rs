// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `ctlz_nonzero` fails if zero is passed as an argument

#![feature(core_intrinsics)]
use std::intrinsics::ctlz_nonzero;

// Call `ctlz_nonzero` with an unconstrained symbolic argument
macro_rules! test_ctlz_nonzero {
    ($ty:ty) => {
        let var_nonzero: $ty = kani::any();
        let _ = unsafe { ctlz_nonzero(var_nonzero) };
    };
}

#[kani::proof]
fn main() {
    test_ctlz_nonzero!(u8);
    test_ctlz_nonzero!(u16);
    test_ctlz_nonzero!(u32);
    test_ctlz_nonzero!(u64);
    test_ctlz_nonzero!(u128);
    test_ctlz_nonzero!(usize);
}
