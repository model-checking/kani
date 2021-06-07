// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]
use std::intrinsics::{atomic_store, atomic_store_rel, atomic_store_relaxed};

fn main() {
    let mut a1 = 1 as u8;
    let mut a2 = 1 as u8;
    let mut a3 = 1 as u8;

    let ptr_a1: *mut u8 = &mut a1;
    let ptr_a2: *mut u8 = &mut a2;
    let ptr_a3: *mut u8 = &mut a3;

    unsafe {
        atomic_store(ptr_a1, 0);
        atomic_store_rel(ptr_a2, 0);
        atomic_store_relaxed(ptr_a3, 0);

        assert!(*ptr_a1 == 0);
        assert!(*ptr_a2 == 0);
        assert!(*ptr_a3 == 0);
    }
}
