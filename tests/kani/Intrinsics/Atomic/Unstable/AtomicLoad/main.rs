// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_load_seqcst` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{atomic_load_acquire, atomic_load_relaxed, atomic_load_seqcst};

#[kani::proof]
fn main() {
    let a1 = 1 as u8;
    let a2 = 1 as u8;
    let a3 = 1 as u8;

    let ptr_a1: *const u8 = &a1;
    let ptr_a2: *const u8 = &a2;
    let ptr_a3: *const u8 = &a3;

    unsafe {
        let x1 = atomic_load_seqcst(ptr_a1);
        let x2 = atomic_load_acquire(ptr_a2);
        let x3 = atomic_load_relaxed(ptr_a3);

        assert!(x1 == 1);
        assert!(x2 == 1);
        assert!(x3 == 1);
    }
}
