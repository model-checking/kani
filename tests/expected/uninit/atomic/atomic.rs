// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#![feature(core_intrinsics)]

use std::alloc::{alloc, Layout};
use std::sync::atomic::{AtomicU8, Ordering};

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
                std::intrinsics::atomic_store_relaxed(ptr, 1);
            }
            1 => {
                std::intrinsics::atomic_load_relaxed(ptr as *const u8);
            }
            _ => {
                std::intrinsics::atomic_cxchg_relaxed_relaxed(ptr, 1, 1);
            }
        };
    }
}
