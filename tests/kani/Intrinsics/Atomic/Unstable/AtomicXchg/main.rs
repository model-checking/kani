// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_xchg_seqcst` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{AtomicOrdering, atomic_xchg};

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

    unsafe {
        // Stores a value if the current value is the same as the old value
        // Returns (val, ok) where
        //  * val: the old value
        //  * ok:  bool indicating whether the operation was successful or not
        assert!(atomic_xchg::<_, { AtomicOrdering::SeqCst }>(ptr_a1, 1) == 0);
        assert!(atomic_xchg::<_, { AtomicOrdering::SeqCst }>(ptr_a1, 0) == 1);
        assert!(atomic_xchg::<_, { AtomicOrdering::Acquire }>(ptr_a2, 1) == 0);
        assert!(atomic_xchg::<_, { AtomicOrdering::Acquire }>(ptr_a2, 0) == 1);
        assert!(atomic_xchg::<_, { AtomicOrdering::AcqRel }>(ptr_a3, 1) == 0);
        assert!(atomic_xchg::<_, { AtomicOrdering::AcqRel }>(ptr_a3, 0) == 1);
        assert!(atomic_xchg::<_, { AtomicOrdering::Release }>(ptr_a4, 1) == 0);
        assert!(atomic_xchg::<_, { AtomicOrdering::Release }>(ptr_a4, 0) == 1);
        assert!(atomic_xchg::<_, { AtomicOrdering::Relaxed }>(ptr_a5, 1) == 0);
        assert!(atomic_xchg::<_, { AtomicOrdering::Relaxed }>(ptr_a5, 0) == 1);
    }
}
