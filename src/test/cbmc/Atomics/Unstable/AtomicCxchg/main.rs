// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_cxchg, atomic_cxchg_acq, atomic_cxchg_acq_failrelaxed, atomic_cxchg_acqrel,
    atomic_cxchg_acqrel_failrelaxed, atomic_cxchg_failacq, atomic_cxchg_failrelaxed,
    atomic_cxchg_rel, atomic_cxchg_relaxed,
};

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

    let ptr_a1: *mut u8 = &mut a1;
    let ptr_a2: *mut u8 = &mut a2;
    let ptr_a3: *mut u8 = &mut a3;
    let ptr_a4: *mut u8 = &mut a4;
    let ptr_a5: *mut u8 = &mut a5;
    let ptr_a6: *mut u8 = &mut a6;
    let ptr_a7: *mut u8 = &mut a7;
    let ptr_a8: *mut u8 = &mut a8;
    let ptr_a9: *mut u8 = &mut a9;

    unsafe {
        // Stores a value if the current value is the same as the old value
        // Returns (val, ok) where
        //  * val: the old value
        //  * ok:  bool indicating whether the operation was successful or not
        let x1 = atomic_cxchg(ptr_a1, 0, 1);
        let x2 = atomic_cxchg_acq(ptr_a2, 0, 1);
        let x3 = atomic_cxchg_acq_failrelaxed(ptr_a3, 0, 1);
        let x4 = atomic_cxchg_acqrel(ptr_a4, 0, 1);
        let x5 = atomic_cxchg_acqrel_failrelaxed(ptr_a5, 0, 1);
        let x6 = atomic_cxchg_failacq(ptr_a6, 0, 1);
        let x7 = atomic_cxchg_failrelaxed(ptr_a7, 0, 1);
        let x8 = atomic_cxchg_rel(ptr_a8, 0, 1);
        let x9 = atomic_cxchg_relaxed(ptr_a9, 0, 1);

        assert!(x1 == (0, true));
        assert!(x2 == (0, true));
        assert!(x3 == (0, true));
        assert!(x4 == (0, true));
        assert!(x5 == (0, true));
        assert!(x6 == (0, true));
        assert!(x7 == (0, true));
        assert!(x8 == (0, true));
        assert!(x9 == (0, true));

        let y1 = atomic_cxchg(ptr_a1, 1, 1);
        let y2 = atomic_cxchg_acq(ptr_a2, 1, 1);
        let y3 = atomic_cxchg_acq_failrelaxed(ptr_a3, 1, 1);
        let y4 = atomic_cxchg_acqrel(ptr_a4, 1, 1);
        let y5 = atomic_cxchg_acqrel_failrelaxed(ptr_a5, 1, 1);
        let y6 = atomic_cxchg_failacq(ptr_a6, 1, 1);
        let y7 = atomic_cxchg_failrelaxed(ptr_a7, 1, 1);
        let y8 = atomic_cxchg_rel(ptr_a8, 1, 1);
        let y9 = atomic_cxchg_relaxed(ptr_a9, 1, 1);

        assert!(y1 == (1, true));
        assert!(y2 == (1, true));
        assert!(y3 == (1, true));
        assert!(y4 == (1, true));
        assert!(y5 == (1, true));
        assert!(y6 == (1, true));
        assert!(y7 == (1, true));
        assert!(y8 == (1, true));
        assert!(y9 == (1, true));
    }
}
