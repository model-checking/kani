// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `atomic_max_rel` is not supported.

#![feature(core_intrinsics)]
use std::intrinsics::atomic_max_rel;

#[kani::proof]
fn main() {
    let mut a1 = 0 as u8;
    let ptr_a1: *mut u8 = &mut a1;

    let b = 1 as u8;

    unsafe {
        let x1 = atomic_max_rel(ptr_a1, b);
        assert!(x1 == 0);
        assert!(*ptr_a1 == 1);
    }
}
