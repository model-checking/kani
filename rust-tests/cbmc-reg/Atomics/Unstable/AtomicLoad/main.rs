// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{atomic_load, atomic_load_acq, atomic_load_relaxed};

fn main() {
    let a1 = 1 as u8;
    let a2 = 1 as u8;
    let a3 = 1 as u8;

    let ptr_a1: *const u8 = &a1;
    let ptr_a2: *const u8 = &a2;
    let ptr_a3: *const u8 = &a3;

    unsafe {
        let x1 = atomic_load(ptr_a1);
        let x2 = atomic_load_acq(ptr_a2);
        let x3 = atomic_load_relaxed(ptr_a3);

        assert!(x1 == 1);
        assert!(x2 == 1);
        assert!(x3 == 1);
    }
}
