// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::sync::atomic::{AtomicUsize, Ordering};

fn any_ordering() -> Ordering {
    match kani::any() {
        0 => Ordering::Relaxed,
        1 => Ordering::Release,
        2 => Ordering::Acquire,
        3 => Ordering::AcqRel,
        _ => Ordering::SeqCst,
    }
}

fn store_ordering() -> Ordering {
    match kani::any() {
        0 => Ordering::Relaxed,
        1 => Ordering::Release,
        _ => Ordering::SeqCst,
    }
}

fn load_ordering() -> Ordering {
    match kani::any() {
        0 => Ordering::Relaxed,
        1 => Ordering::Acquire,
        _ => Ordering::SeqCst,
    }
}

static GLOBAL_ATOMIC: AtomicUsize = AtomicUsize::new(0);

// Checks if memory initialization checks work with atomics defined in the global scope.
#[kani::proof]
fn global_atomic() {
    let old_value = GLOBAL_ATOMIC.fetch_add(1, any_ordering());
}

// Checks if memory initialization checks work with atomics.
#[kani::proof]
fn local_atomic() {
    // Get a pointer to an allocated value
    let ptr: *mut usize = Box::into_raw(Box::new(0));

    // Create an atomic from the allocated value
    let atomic = unsafe { AtomicUsize::from_ptr(ptr) };

    // Use `atomic` for atomic operations
    atomic.store(1, store_ordering());
    let old_val = atomic.load(load_ordering());
    let old_val = atomic.swap(2, any_ordering());

    // Deallocate the value
    unsafe { drop(Box::from_raw(ptr)) }
}

// Checks if memory initialization checks work with compare-and-swap atomics.
#[kani::proof]
fn compare_exchange_atomic() {
    let some_var = AtomicUsize::new(5);
    some_var.compare_exchange(5, 10, any_ordering(), load_ordering());
    let val = some_var.load(load_ordering());
    assert!(val == 10);
}
