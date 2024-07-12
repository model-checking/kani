// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks
//! Checks that Kani supports memory initialization checks via intrinsics.

#![feature(core_intrinsics)]

use std::alloc::{alloc, alloc_zeroed, Layout};
use std::intrinsics::*;

#[kani::proof]
fn check_copy_nonoverlapping() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let src: *mut u8 = alloc(layout);
        let dst: *mut u8 = alloc(layout);
        copy_nonoverlapping(src as *const u8, dst, 2); // ~ERROR: Accessing `src` here, which is uninitialized.
    }
}

#[kani::proof]
fn check_copy_nonoverlapping_safe() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let src: *mut u8 = alloc_zeroed(layout);
        let dst: *mut u8 = alloc(layout);
        // `src` is initialized here, `dst` is uninitialized, but it is fine since we are writing into it.
        copy_nonoverlapping(src as *const u8, dst, 2);
    }
}

#[kani::proof]
fn check_copy() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let src: *mut u8 = alloc(layout);
        let dst: *mut u8 = alloc(layout);
        copy(src as *const u8, dst, 2); // ~ERROR: Accessing `src` here, which is uninitialized.
    }
}

#[kani::proof]
fn check_copy_safe() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let src: *mut u8 = alloc_zeroed(layout);
        let dst: *mut u8 = alloc(layout);
        // `src` is initialized here, `dst` is uninitialized, but it is fine since we are writing into it.
        copy(src as *const u8, dst, 2);
    }
}

#[kani::proof]
fn check_compare_bytes() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let left: *mut u8 = alloc(layout);
        let right: *mut u8 = alloc(layout);
        // ~ERROR: Accessing `left` and `right` here, both of which are uninitialized.
        compare_bytes(left as *const u8, right as *const u8, 2);
    }
}

#[kani::proof]
fn check_compare_bytes_safe() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let left: *mut u8 = alloc_zeroed(layout);
        let right: *mut u8 = alloc_zeroed(layout);
        // Both `left` and `right` are initialized here.
        compare_bytes(left as *const u8, right as *const u8, 2);
    }
}

#[kani::proof]
fn check_write_bytes_safe() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let left: *mut u8 = alloc(layout);
        let right: *mut u8 = alloc(layout);
        write_bytes(left, 0, 2);
        write_bytes(right, 0, 2);
        // Both `left` and `right` are initialized here.
        compare_bytes(left as *const u8, right as *const u8, 2);
    }
}

#[kani::proof]
fn check_volatile_load() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let src: *mut u8 = alloc(layout);
        volatile_load(src as *const u8); // ~ERROR: Accessing `src` here, which is uninitialized.
    }
}

#[kani::proof]
fn check_volatile_store_and_load_safe() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let src: *mut u8 = alloc(layout);
        volatile_store(src, 0);
        volatile_load(src as *const u8); // `src` is initialized here.
    }
}

#[kani::proof]
fn check_typed_swap() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let left: *mut u8 = alloc(layout);
        let right: *mut u8 = alloc(layout);
        // ~ERROR: Accessing `left` and `right` here, both of which are uninitialized.
        typed_swap(left, right);
    }
}

#[kani::proof]
fn check_typed_swap_safe() {
    unsafe {
        let layout = Layout::from_size_align(16, 8).unwrap();
        let left: *mut u8 = alloc_zeroed(layout);
        let right: *mut u8 = alloc_zeroed(layout);
        // Both `left` and `right` are initialized here.
        typed_swap(left, right);
    }
}
