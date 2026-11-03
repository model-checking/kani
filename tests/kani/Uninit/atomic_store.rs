// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#![feature(core_intrinsics)]
#![feature(min_adt_const_params)]

use std::alloc::{Layout, alloc, dealloc};
use std::intrinsics::{AtomicOrdering, atomic_load, atomic_store, atomic_xchg};

fn store_initializes<const ORD: AtomicOrdering>() {
    let layout = Layout::new::<u64>();
    let ptr = unsafe { alloc(layout) }.cast::<u64>();
    kani::assume(!ptr.is_null());
    let value: u64 = kani::any();
    unsafe {
        atomic_store::<u64, ORD, false>(ptr, value);
        assert_eq!(atomic_load::<u64, { AtomicOrdering::Relaxed }, false>(ptr), value);
        dealloc(ptr.cast(), layout);
    }
}

#[kani::proof]
fn store_initializes_memory() {
    store_initializes::<{ AtomicOrdering::Relaxed }>();
    store_initializes::<{ AtomicOrdering::Release }>();
    store_initializes::<{ AtomicOrdering::SeqCst }>();
}

#[kani::proof]
fn store_initializes_pointer() {
    let layout = Layout::new::<*mut u8>();
    let ptr = unsafe { alloc(layout) }.cast::<*mut u8>();
    kani::assume(!ptr.is_null());
    let mut pointee: u8 = kani::any();
    let value = if kani::any() { &raw mut pointee } else { std::ptr::null_mut() };
    unsafe {
        atomic_store::<*mut u8, { AtomicOrdering::SeqCst }, false>(ptr, value);
        let loaded = atomic_load::<*mut u8, { AtomicOrdering::SeqCst }, false>(ptr);
        assert_eq!(loaded, value);
        if !loaded.is_null() {
            assert_eq!(*loaded, pointee);
        }
        dealloc(ptr.cast(), layout);
    }
}

#[kani::proof]
fn exchange_returns_previous_value() {
    let mut value: u64 = kani::any();
    let old = value;
    let new: u64 = kani::any();
    let returned = unsafe { atomic_xchg::<u64, { AtomicOrdering::SeqCst }>(&raw mut value, new) };
    assert_eq!(returned, old);
    assert_eq!(value, new);
}
