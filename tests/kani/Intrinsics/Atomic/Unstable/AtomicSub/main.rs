// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_xsub` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_xsub, atomic_xsub_acq, atomic_xsub_acqrel, atomic_xsub_rel, atomic_xsub_relaxed,
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

    let b = 1 as u8;
    let c = 0 as u8;

    unsafe {
        let x1 = atomic_xsub(ptr_a1, b);
        let x2 = atomic_xsub_acq(ptr_a2, b);
        let x3 = atomic_xsub_acqrel(ptr_a3, b);
        let x4 = atomic_xsub_rel(ptr_a4, b);
        let x5 = atomic_xsub_relaxed(ptr_a5, b);

        assert!(x1 == 1);
        assert!(x2 == 1);
        assert!(x3 == 1);
        assert!(x4 == 1);
        assert!(x5 == 1);

        assert!(*ptr_a1 == c);
        assert!(*ptr_a2 == c);
        assert!(*ptr_a3 == c);
        assert!(*ptr_a4 == c);
        assert!(*ptr_a5 == c);
    }
}
