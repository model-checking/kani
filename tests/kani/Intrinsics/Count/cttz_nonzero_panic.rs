// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `cttz_nonzero` fails if zero is passed as an argument

#![feature(core_intrinsics)]
use std::intrinsics::cttz_nonzero;

// Call `cttz_nonzero` with an unconstrained symbolic argument
macro_rules! test_cttz_nonzero {
    ($ty:ty) => {
        let var_nonzero: $ty = kani::any();
        let _ = unsafe { cttz_nonzero(var_nonzero) };
    };
}

#[kani::proof]
fn main() {
    test_cttz_nonzero!(u8);
    test_cttz_nonzero!(u16);
    test_cttz_nonzero!(u32);
    test_cttz_nonzero!(u64);
    test_cttz_nonzero!(u128);
    test_cttz_nonzero!(usize);
}
