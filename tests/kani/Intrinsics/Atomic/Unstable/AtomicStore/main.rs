// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `atomic_store_seqcst` and other variants (unstable version) return the
// expected result.

#![feature(core_intrinsics)]
use std::intrinsics::{AtomicOrdering, atomic_store};

#[kani::proof]
fn main() {
    let mut a1 = 1 as u8;
    let mut a2 = 1 as u8;
    let mut a3 = 1 as u8;

    let ptr_a1: *mut u8 = &mut a1;
    let ptr_a2: *mut u8 = &mut a2;
    let ptr_a3: *mut u8 = &mut a3;

    unsafe {
        atomic_store::<_, { AtomicOrdering::SeqCst }>(ptr_a1, 0);
        atomic_store::<_, { AtomicOrdering::Release }>(ptr_a2, 0);
        atomic_store::<_, { AtomicOrdering::Relaxed }>(ptr_a3, 0);

        assert!(*ptr_a1 == 0);
        assert!(*ptr_a2 == 0);
        assert!(*ptr_a3 == 0);
    }
}
