// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{
    atomic_or, atomic_or_acq, atomic_or_acqrel, atomic_or_rel, atomic_or_relaxed,
};

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
        let x1 = atomic_or(ptr_a1, b);
        let x2 = atomic_or_acq(ptr_a2, b);
        let x3 = atomic_or_acqrel(ptr_a3, b);
        let x4 = atomic_or_rel(ptr_a4, b);
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
