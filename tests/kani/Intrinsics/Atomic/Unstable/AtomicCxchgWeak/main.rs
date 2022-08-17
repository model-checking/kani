// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that all variants of the `atomic_cxchgweak_*` intrinsic (unstable
// version) return the expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_cxchgweak_acqrel_acquire, atomic_cxchgweak_acqrel_relaxed,
    atomic_cxchgweak_acqrel_seqcst, atomic_cxchgweak_acquire_acquire,
    atomic_cxchgweak_acquire_relaxed, atomic_cxchgweak_acquire_seqcst,
    atomic_cxchgweak_relaxed_acquire, atomic_cxchgweak_relaxed_relaxed,
    atomic_cxchgweak_relaxed_seqcst, atomic_cxchgweak_release_acquire,
    atomic_cxchgweak_release_relaxed, atomic_cxchgweak_release_seqcst,
    atomic_cxchgweak_seqcst_acquire, atomic_cxchgweak_seqcst_relaxed,
    atomic_cxchgweak_seqcst_seqcst,
};

#[kani::proof]
fn main() {
    let mut a1 = 0 as u8;
    let mut a2 = 0 as u8;
    let mut a3 = 0 as u8;
    let mut a4 = 0 as u8;
    let mut a5 = 0 as u8;
    let mut a6 = 0 as u8;
    let mut a7 = 0 as u8;
    let mut a8 = 0 as u8;
    let mut a9 = 0 as u8;
    let mut a10 = 0 as u8;
    let mut a11 = 0 as u8;
    let mut a12 = 0 as u8;
    let mut a13 = 0 as u8;
    let mut a14 = 0 as u8;
    let mut a15 = 0 as u8;

    let ptr_a1: *mut u8 = &mut a1;
    let ptr_a2: *mut u8 = &mut a2;
    let ptr_a3: *mut u8 = &mut a3;
    let ptr_a4: *mut u8 = &mut a4;
    let ptr_a5: *mut u8 = &mut a5;
    let ptr_a6: *mut u8 = &mut a6;
    let ptr_a7: *mut u8 = &mut a7;
    let ptr_a8: *mut u8 = &mut a8;
    let ptr_a9: *mut u8 = &mut a9;
    let ptr_a10: *mut u8 = &mut a10;
    let ptr_a11: *mut u8 = &mut a11;
    let ptr_a12: *mut u8 = &mut a12;
    let ptr_a13: *mut u8 = &mut a13;
    let ptr_a14: *mut u8 = &mut a14;
    let ptr_a15: *mut u8 = &mut a15;

    unsafe {
        // Stores a value if the current value is the same as the old value
        // Returns (val, ok) where
        //  * val: the old value
        //  * ok:  bool indicating whether the operation was successful or not
        let x1 = atomic_cxchgweak_acqrel_acquire(ptr_a1, 0, 1);
        let x2 = atomic_cxchgweak_acqrel_relaxed(ptr_a2, 0, 1);
        let x3 = atomic_cxchgweak_acqrel_seqcst(ptr_a3, 0, 1);
        let x4 = atomic_cxchgweak_acquire_acquire(ptr_a4, 0, 1);
        let x5 = atomic_cxchgweak_acquire_relaxed(ptr_a5, 0, 1);
        let x6 = atomic_cxchgweak_acquire_seqcst(ptr_a6, 0, 1);
        let x7 = atomic_cxchgweak_relaxed_acquire(ptr_a7, 0, 1);
        let x8 = atomic_cxchgweak_relaxed_relaxed(ptr_a8, 0, 1);
        let x9 = atomic_cxchgweak_relaxed_seqcst(ptr_a9, 0, 1);
        let x10 = atomic_cxchgweak_release_acquire(ptr_a10, 0, 1);
        let x11 = atomic_cxchgweak_release_relaxed(ptr_a11, 0, 1);
        let x12 = atomic_cxchgweak_release_seqcst(ptr_a12, 0, 1);
        let x13 = atomic_cxchgweak_seqcst_acquire(ptr_a13, 0, 1);
        let x14 = atomic_cxchgweak_seqcst_relaxed(ptr_a14, 0, 1);
        let x15 = atomic_cxchgweak_seqcst_seqcst(ptr_a15, 0, 1);

        assert!(x1 == (0, true));
        assert!(x2 == (0, true));
        assert!(x3 == (0, true));
        assert!(x4 == (0, true));
        assert!(x5 == (0, true));
        assert!(x6 == (0, true));
        assert!(x7 == (0, true));
        assert!(x8 == (0, true));
        assert!(x9 == (0, true));
        assert!(x10 == (0, true));
        assert!(x11 == (0, true));
        assert!(x12 == (0, true));
        assert!(x13 == (0, true));
        assert!(x14 == (0, true));
        assert!(x15 == (0, true));

        let y1 = atomic_cxchgweak_acqrel_acquire(ptr_a1, 1, 1);
        let y2 = atomic_cxchgweak_acqrel_relaxed(ptr_a2, 1, 1);
        let y3 = atomic_cxchgweak_acqrel_seqcst(ptr_a3, 1, 1);
        let y4 = atomic_cxchgweak_acquire_acquire(ptr_a4, 1, 1);
        let y5 = atomic_cxchgweak_acquire_relaxed(ptr_a5, 1, 1);
        let y6 = atomic_cxchgweak_acquire_seqcst(ptr_a6, 1, 1);
        let y7 = atomic_cxchgweak_relaxed_acquire(ptr_a7, 1, 1);
        let y8 = atomic_cxchgweak_relaxed_relaxed(ptr_a8, 1, 1);
        let y9 = atomic_cxchgweak_relaxed_seqcst(ptr_a9, 1, 1);
        let y10 = atomic_cxchgweak_release_acquire(ptr_a10, 1, 1);
        let y11 = atomic_cxchgweak_release_relaxed(ptr_a11, 1, 1);
        let y12 = atomic_cxchgweak_release_seqcst(ptr_a12, 1, 1);
        let y13 = atomic_cxchgweak_seqcst_acquire(ptr_a13, 1, 1);
        let y14 = atomic_cxchgweak_seqcst_relaxed(ptr_a14, 1, 1);
        let y15 = atomic_cxchgweak_seqcst_seqcst(ptr_a15, 1, 1);

        assert!(y1 == (1, true));
        assert!(y2 == (1, true));
        assert!(y3 == (1, true));
        assert!(y4 == (1, true));
        assert!(y5 == (1, true));
        assert!(y6 == (1, true));
        assert!(y7 == (1, true));
        assert!(y8 == (1, true));
        assert!(y9 == (1, true));
        assert!(y10 == (1, true));
        assert!(y11 == (1, true));
        assert!(y12 == (1, true));
        assert!(y13 == (1, true));
        assert!(y14 == (1, true));
        assert!(y15 == (1, true));
    }
}
