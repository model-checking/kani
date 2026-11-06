// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#![feature(core_intrinsics)]

use std::alloc::{Layout, alloc, dealloc};
use std::intrinsics::{AtomicOrdering, atomic_cxchg, atomic_load, atomic_store, atomic_xchg};

// Checks if memory initialization checks correctly fail when uninitialized memory is passed to
// atomic intrinsics.
#[kani::proof]
fn local_atomic_uninit() {
    // Get a pointer to an uninitialized value
    let layout = Layout::from_size_align(16, 8).unwrap();
    let ptr: *mut u8 = unsafe { alloc(layout) };
    kani::assume(!ptr.is_null());
    // Try accessing `ptr` via atomic intrinsics, should be UB in each case.
    unsafe {
        match kani::any() {
            0 => {
                atomic_xchg::<_, { AtomicOrdering::Relaxed }>(ptr, 1);
            }
            1 => {
                atomic_load::<_, { AtomicOrdering::Relaxed }, /* VOLATILE */ false>(
                    ptr as *const u8,
                );
            }
            _ => {
                atomic_cxchg::<_, { AtomicOrdering::Relaxed }, { AtomicOrdering::Relaxed }>(
                    ptr, 1, 1,
                );
            }
        };
    }
}

#[kani::proof]
fn atomic_store_null() {
    unsafe {
        atomic_store::<u8, { AtomicOrdering::Relaxed }, false>(std::ptr::null_mut(), 1);
    }
}

#[kani::proof]
fn volatile_atomic_store_null() {
    unsafe {
        atomic_store::<u8, { AtomicOrdering::Relaxed }, true>(std::ptr::null_mut(), 1);
    }
}

#[kani::proof]
fn store_does_not_initialize_next_value() {
    let layout = Layout::array::<std::sync::atomic::AtomicU64>(2).unwrap();
    let ptr = unsafe { alloc(layout) }.cast::<u64>();
    kani::assume(!ptr.is_null());
    unsafe {
        atomic_store::<u64, { AtomicOrdering::Relaxed }, true>(ptr, 1);
        atomic_load::<u64, { AtomicOrdering::Relaxed }, false>(ptr.add(1));
        dealloc(ptr.cast(), layout);
    }
}

#[kani::proof]
fn nonvolatile_store_preserves_initialization_check() {
    // Preserve Kani's existing requirement while the `from_ptr` contract is unresolved.
    let layout = Layout::new::<std::sync::atomic::AtomicU64>();
    let ptr = unsafe { alloc(layout) }.cast::<u64>();
    kani::assume(!ptr.is_null());
    unsafe {
        atomic_store::<u64, { AtomicOrdering::Relaxed }, false>(ptr, 1);
        dealloc(ptr.cast(), layout);
    }
}
