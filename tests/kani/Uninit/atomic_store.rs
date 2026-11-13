// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#![feature(core_intrinsics)]
#![feature(min_adt_const_params)]

use std::alloc::{Layout, alloc, dealloc};
use std::intrinsics::{AtomicOrdering, atomic_load, atomic_store, atomic_xchg};
use std::sync::atomic::{AtomicPtr, AtomicU64};

fn volatile_store_initializes<const ORD: AtomicOrdering>() {
    let layout = Layout::new::<AtomicU64>();
    let ptr = unsafe { alloc(layout) }.cast::<u64>();
    kani::assume(!ptr.is_null());
    let value: u64 = kani::any();
    unsafe {
        atomic_store::<u64, ORD, true>(ptr, value);
        assert_eq!(atomic_load::<u64, { AtomicOrdering::Relaxed }, false>(ptr), value);
        dealloc(ptr.cast(), layout);
    }
}

#[kani::proof]
fn volatile_store_initializes_memory() {
    volatile_store_initializes::<{ AtomicOrdering::Relaxed }>();
    volatile_store_initializes::<{ AtomicOrdering::Release }>();
    volatile_store_initializes::<{ AtomicOrdering::SeqCst }>();
}

#[kani::proof]
fn volatile_store_initializes_pointer() {
    let layout = Layout::new::<AtomicPtr<u8>>();
    let ptr = unsafe { alloc(layout) }.cast::<*mut u8>();
    kani::assume(!ptr.is_null());
    let mut pointee: u8 = kani::any();
    let value = if kani::any() { &raw mut pointee } else { std::ptr::null_mut() };
    unsafe {
        atomic_store::<*mut u8, { AtomicOrdering::SeqCst }, true>(ptr, value);
        let loaded = atomic_load::<*mut u8, { AtomicOrdering::SeqCst }, false>(ptr);
        assert_eq!(loaded, value);
        if !loaded.is_null() {
            assert_eq!(*loaded, pointee);
        }
        dealloc(ptr.cast(), layout);
    }
}

#[kani::proof]
fn nonvolatile_store_updates_initialized_memory() {
    let atomic = AtomicU64::new(kani::any());
    let value: u64 = kani::any();
    let pointer = AtomicPtr::<u8>::new(std::ptr::null_mut());
    let mut pointee: u8 = kani::any();
    unsafe {
        atomic_store::<u64, { AtomicOrdering::Relaxed }, false>(atomic.as_ptr(), value);
        assert_eq!(atomic_load::<u64, { AtomicOrdering::Relaxed }, false>(atomic.as_ptr()), value);
        atomic_store::<*mut u8, { AtomicOrdering::Release }, false>(
            pointer.as_ptr(),
            &raw mut pointee,
        );
        let loaded = atomic_load::<*mut u8, { AtomicOrdering::Acquire }, false>(pointer.as_ptr());
        assert_eq!(*loaded, pointee);
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
