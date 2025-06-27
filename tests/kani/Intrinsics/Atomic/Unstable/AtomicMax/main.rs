// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_max_seqcst` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{AtomicOrdering, atomic_max};

#[kani::proof]
fn main() {
    let mut a1 = 0 as u8;
    let mut a2 = 0 as u8;
    let mut a3 = 0 as u8;
    let mut a4 = 0 as u8;
    let mut a5 = 0 as u8;

    let ptr_a1: *mut u8 = &mut a1;
    let ptr_a2: *mut u8 = &mut a2;
    let ptr_a3: *mut u8 = &mut a3;
    let ptr_a4: *mut u8 = &mut a4;
    let ptr_a5: *mut u8 = &mut a5;

    let b = 1 as u8;

    unsafe {
        let x1 = atomic_max::<_, { AtomicOrdering::SeqCst }>(ptr_a1, b);
        let x2 = atomic_max::<_, { AtomicOrdering::Acquire }>(ptr_a2, b);
        let x3 = atomic_max::<_, { AtomicOrdering::AcqRel }>(ptr_a3, b);
        let x4 = atomic_max::<_, { AtomicOrdering::Release }>(ptr_a4, b);
        let x5 = atomic_max::<_, { AtomicOrdering::Relaxed }>(ptr_a5, b);

        assert!(x1 == 0);
        assert!(x2 == 0);
        assert!(x3 == 0);
        assert!(x4 == 0);
        assert!(x5 == 0);

        assert!(*ptr_a1 == b);
        assert!(*ptr_a2 == b);
        assert!(*ptr_a3 == b);
        assert!(*ptr_a4 == b);
        assert!(*ptr_a5 == b);
    }
}
