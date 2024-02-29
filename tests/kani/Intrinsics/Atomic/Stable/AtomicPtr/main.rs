// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Test atomic intrinsics through the stable interface of atomic_ptr.
// Specifically, it checks that Kani correctly handles atomic_ptr's fetch methods, in which the second argument is a pointer type.
// These methods were not correctly handled as explained in https://github.com/model-checking/kani/issues/3042.

#![feature(strict_provenance_atomic_ptr, strict_provenance)]
use std::sync::atomic::{AtomicPtr, Ordering};

#[kani::proof]
fn check_fetch_byte_add() {
    let atom = AtomicPtr::<i64>::new(core::ptr::null_mut());
    assert_eq!(atom.fetch_byte_add(1, Ordering::Relaxed).addr(), 0);
    // Note: in units of bytes, not `size_of::<i64>()`.
    assert_eq!(atom.load(Ordering::Relaxed).addr(), 1);
}

#[kani::proof]
fn check_fetch_byte_sub() {
    let atom = AtomicPtr::<i64>::new(core::ptr::without_provenance_mut(1));
    assert_eq!(atom.fetch_byte_sub(1, Ordering::Relaxed).addr(), 1);
    assert_eq!(atom.load(Ordering::Relaxed).addr(), 0);
}

#[kani::proof]
fn check_fetch_and() {
    let pointer = &mut 3i64 as *mut i64;
    // A tagged pointer
    let atom = AtomicPtr::<i64>::new(pointer.map_addr(|a| a | 1));
    assert_eq!(atom.fetch_or(1, Ordering::Relaxed).addr() & 1, 1);
    // Untag, and extract the previously tagged pointer.
    let untagged = atom.fetch_and(!1, Ordering::Relaxed).map_addr(|a| a & !1);
    assert_eq!(untagged, pointer);
}

#[kani::proof]
fn check_fetch_or() {
    let pointer = &mut 3i64 as *mut i64;

    let atom = AtomicPtr::<i64>::new(pointer);
    // Tag the bottom bit of the pointer.
    assert_eq!(atom.fetch_or(1, Ordering::Relaxed).addr() & 1, 0);
    // Extract and untag.
    let tagged = atom.load(Ordering::Relaxed);
    assert_eq!(tagged.addr() & 1, 1);
    assert_eq!(tagged.map_addr(|p| p & !1), pointer);
}

#[kani::proof]
fn check_fetch_update() {
    let ptr: *mut _ = &mut 5;
    let some_ptr = AtomicPtr::new(ptr);

    let new: *mut _ = &mut 10;
    assert_eq!(some_ptr.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| None), Err(ptr));
    let result = some_ptr.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
        if x == ptr { Some(new) } else { None }
    });
    assert_eq!(result, Ok(ptr));
    assert_eq!(some_ptr.load(Ordering::SeqCst), new);
}

#[kani::proof]
fn check_fetch_xor() {
    let pointer = &mut 3i64 as *mut i64;
    let atom = AtomicPtr::<i64>::new(pointer);

    // Toggle a tag bit on the pointer.
    atom.fetch_xor(1, Ordering::Relaxed);
    assert_eq!(atom.load(Ordering::Relaxed).addr() & 1, 1);
}
