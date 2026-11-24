// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Init state survives `realloc` and untyped copies; unwritten bytes stay uninitialized.

use std::alloc::{Layout, alloc, alloc_zeroed, dealloc, realloc};
use std::ptr;

#[kani::proof]
fn check_realloc_grow_keeps_prefix() {
    unsafe {
        let layout = Layout::from_size_align(4, 1).unwrap();
        let p = alloc(layout);
        ptr::write_bytes(p, 7, 4);
        let q = realloc(p, layout, 8);
        let idx: usize = kani::any();
        kani::assume(idx < 4);
        assert!(*q.add(idx) == 7);
        dealloc(q, Layout::from_size_align(8, 1).unwrap());
    }
}

#[kani::proof]
fn check_realloc_shrink_keeps_prefix() {
    unsafe {
        let layout = Layout::from_size_align(8, 1).unwrap();
        let p = alloc(layout);
        ptr::write_bytes(p, 3, 8);
        let q = realloc(p, layout, 2);
        assert!(*q == 3 && *q.add(1) == 3);
        dealloc(q, Layout::from_size_align(2, 1).unwrap());
    }
}

#[kani::proof]
fn check_realloc_grow_tail_uninit() {
    unsafe {
        let layout = Layout::from_size_align(4, 1).unwrap();
        let p = alloc(layout);
        ptr::write_bytes(p, 7, 4);
        let q = realloc(p, layout, 8);
        let _byte = *q.add(5); // ~ERROR: byte 5 was never written.
    }
}

#[kani::proof]
fn check_realloc_carries_uninit() {
    unsafe {
        let layout = Layout::from_size_align(4, 1).unwrap();
        let p = alloc(layout);
        *p = 1;
        *p.add(1) = 1;
        *p.add(3) = 1;
        let q = realloc(p, layout, 8);
        let _byte = *q.add(2); // ~ERROR: byte 2 was never written.
    }
}

#[kani::proof]
fn check_copy_into_fresh_allocation() {
    unsafe {
        let src: [u16; 4] = [1, 2, 3, 4];
        let layout = Layout::array::<u16>(4).unwrap();
        let dst = alloc(layout) as *mut u16;
        ptr::copy_nonoverlapping(src.as_ptr(), dst, 4);
        let idx: usize = kani::any();
        kani::assume(idx < 4);
        assert!(*dst.add(idx) == src[idx]);
        dealloc(dst as *mut u8, layout);
    }
}

#[kani::proof]
fn check_alloc_zeroed_initialized() {
    unsafe {
        let layout = Layout::from_size_align(8, 4).unwrap();
        let p = alloc_zeroed(layout);
        let idx: usize = kani::any();
        kani::assume(idx < 8);
        assert!(*p.add(idx) == 0);
        dealloc(p, layout);
    }
}

#[kani::proof]
fn check_alloc_zeroed_then_realloc() {
    unsafe {
        let layout = Layout::from_size_align(4, 1).unwrap();
        let p = alloc_zeroed(layout);
        let q = realloc(p, layout, 16);
        let idx: usize = kani::any();
        kani::assume(idx < 4);
        assert!(*q.add(idx) == 0);
        dealloc(q, Layout::from_size_align(16, 1).unwrap());
    }
}

#[kani::proof]
fn check_realloc_wide_elements_keep_prefix() {
    unsafe {
        let layout = Layout::array::<u32>(2).unwrap();
        let p = alloc(layout) as *mut u32;
        *p = 5;
        *p.add(1) = 6;
        let q = realloc(p as *mut u8, layout, 16) as *mut u32;
        let idx: usize = kani::any();
        kani::assume(idx < 2);
        assert!(*q.add(idx) == 5 + idx as u32);
        dealloc(q as *mut u8, Layout::array::<u32>(4).unwrap());
    }
}

#[kani::proof]
fn check_realloc_wide_elements_carry_uninit() {
    unsafe {
        let layout = Layout::array::<u32>(2).unwrap();
        let p = alloc(layout) as *mut u32;
        *p = 5;
        let q = realloc(p as *mut u8, layout, 16) as *mut u32;
        let _elem = *q.add(1); // ~ERROR: element 1 was never written.
    }
}
