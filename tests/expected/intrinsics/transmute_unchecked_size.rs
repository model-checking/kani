// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that Kani correctly identify UB when invoking `transmute_unchecked` with different sizes.
//! See <https://github.com/model-checking/kani/issues/3839> for more details.

#![feature(core_intrinsics)]
use std::intrinsics::transmute_unchecked;

/// Reachability doesn't seem to work for unreachable statements.
macro_rules! unreachable {
    ($msg:literal) => {
        assert!(false, $msg)
    };
}

/// This should fail due to UB detection.
#[kani::proof]
pub fn transmute_diff_size() {
    let a: u32 = kani::any();
    if kani::any() {
        let smaller: u16 = unsafe { transmute_unchecked(a) };
        std::hint::black_box(smaller);
        unreachable!("This should never be reached");
    } else {
        let bigger: (u64, isize) = unsafe { transmute_unchecked(a) };
        std::hint::black_box(bigger);
        unreachable!("Neither this one");
    }
}

/// Generic transmute wrapper.
pub unsafe fn generic_transmute<S, D>(src: S) -> D {
    transmute_unchecked(src)
}

/// This should also fail due to UB detection.
#[kani::proof]
pub fn transmute_wrapper_diff_size() {
    let a: (u32, char) = kani::any();
    let b: u128 = unsafe { generic_transmute(a) };
    std::hint::black_box(b);
    unreachable!("Unreachable expected");
}
