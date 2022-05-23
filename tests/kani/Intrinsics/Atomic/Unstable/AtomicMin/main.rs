// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_min` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_min, atomic_min_acq, atomic_min_acqrel, atomic_min_rel, atomic_min_relaxed,
};

#[kani::proof]
fn main() {
    let mut a1 = 1 as u8;
    let mut a2 = 1 as u8;
    let mut a3 = 1 as u8;
    let mut a4 = 1 as u8;
    let mut a5 = 1 as u8;

    let ptr_a1: *mut u8 = &mut a1;
    let ptr_a2: *mut u8 = &mut a2;
    let ptr_a3: *mut u8 = &mut a3;
    let ptr_a4: *mut u8 = &mut a4;
    let ptr_a5: *mut u8 = &mut a5;

    let b = 0 as u8;

    unsafe {
        let x1 = atomic_min(ptr_a1, b);
        let x2 = atomic_min_acq(ptr_a2, b);
        let x3 = atomic_min_acqrel(ptr_a3, b);
        let x4 = atomic_min_rel(ptr_a4, b);
        let x5 = atomic_min_relaxed(ptr_a5, b);

        assert!(x1 == 1);
        assert!(x2 == 1);
        assert!(x3 == 1);
        assert!(x4 == 1);
        assert!(x5 == 1);

        assert!(*ptr_a1 == b);
        assert!(*ptr_a2 == b);
        assert!(*ptr_a3 == b);
        assert!(*ptr_a4 == b);
        assert!(*ptr_a5 == b);
    }
}
