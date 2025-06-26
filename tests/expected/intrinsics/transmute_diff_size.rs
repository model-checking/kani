// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that compilation fails when trying to transmute with different src and target sizes.

#![feature(core_intrinsics)]
use std::intrinsics::transmute;

/// This should fail due to UB detection.
#[kani::proof]
pub fn transmute_diff_size() {
    let a: u32 = kani::any();
    if kani::any() {
        let smaller: u16 = unsafe { transmute(a) };
        std::hint::black_box(smaller);
    } else {
        let bigger: (u64, isize) = unsafe { transmute(a) };
        std::hint::black_box(bigger);
    }
}

/// Generic transmute wrapper.
pub unsafe fn generic_transmute<S, D>(src: S) -> D {
    transmute(src)
}

/// This should also fail due to UB detection.
#[kani::proof]
pub fn transmute_wrapper_diff_size() {
    let a: (u32, char) = kani::any();
    let b: u128 = unsafe { generic_transmute(a) };
    std::hint::black_box(b);
}
