// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#![feature(core_intrinsics)]

use std::alloc::{Layout, alloc};
use std::intrinsics::{AtomicOrdering, atomic_cxchg, atomic_load, atomic_store};

// Checks if memory initialization checks correctly fail when uninitialized memory is passed to
// atomic intrinsics.
#[kani::proof]
fn local_atomic_uninit() {
    // Get a pointer to an uninitialized value
    let layout = Layout::from_size_align(16, 8).unwrap();
    let ptr: *mut u8 = unsafe { alloc(layout) };
    // Try accessing `ptr` via atomic intrinsics, should be UB in each case.
    unsafe {
        match kani::any() {
            0 => {
                atomic_store::<_, { AtomicOrdering::Relaxed }>(ptr, 1);
            }
            1 => {
                atomic_load::<_, { AtomicOrdering::Relaxed }>(ptr as *const u8);
            }
            _ => {
                atomic_cxchg::<_, { AtomicOrdering::Relaxed }, { AtomicOrdering::Relaxed }>(
                    ptr, 1, 1,
                );
            }
        };
    }
}
