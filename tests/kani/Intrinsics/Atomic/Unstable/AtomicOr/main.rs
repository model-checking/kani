// Copyright Kani Contributor_seqcsts
// SPDX-License-Identifier: Apache-2.0 OR_seqcst MIT

// Check that `atomic_or_seqcst` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_or_acqrel, atomic_or_acquire, atomic_or_relaxed, atomic_or_release, atomic_or_seqcst,
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
    let c = 1 as u8;

    unsafe {
        let x1 = atomic_or_seqcst(ptr_a1, b);
        let x2 = atomic_or_acquire(ptr_a2, b);
        let x3 = atomic_or_acqrel(ptr_a3, b);
        let x4 = atomic_or_release(ptr_a4, b);
        let x5 = atomic_or_relaxed(ptr_a5, b);

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
