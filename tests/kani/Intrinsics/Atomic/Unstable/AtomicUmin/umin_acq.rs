// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `atomic_umin_acq` is not supported.

#![feature(core_intrinsics)]
use std::intrinsics::atomic_umin_acq;

#[kani::proof]
fn main() {
    let mut a1 = 1 as u8;
    let ptr_a1: *mut u8 = &mut a1;

    let b = 0 as u8;

    unsafe {
        let x1 = atomic_umin_acq(ptr_a1, b);
        assert!(x1 == 1);
        assert!(*ptr_a1 == 0);
    }
}
