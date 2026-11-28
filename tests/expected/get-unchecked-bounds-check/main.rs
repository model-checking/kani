// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani detects an out-of-bounds index passed to `get_unchecked` and
//! `get_unchecked_mut`, which is undefined behavior even when the index stays inside the
//! enclosing allocation.
//! See <https://github.com/model-checking/kani/issues/5155>.
#![feature(slice_ptr_get)]

use std::hint::black_box as bb;

/// The reproducer of issue #3707: a raw slice pointer, index far past the end of the array.
#[kani::proof]
fn raw_slice_mut_far_out_of_bounds() {
    let mut arr = [0i32; 10];
    let ptr = &mut arr as *mut [i32];
    let _ = unsafe { ptr.get_unchecked_mut(bb(100)) };
}

/// Index one past the end of a raw slice pointer.
#[kani::proof]
fn raw_slice_const_one_past_the_end() {
    let arr = [0i32; 10];
    let ptr = &arr as *const [i32];
    let _ = unsafe { ptr.get_unchecked(bb(10)) };
}

/// The index is past the end of the subslice but inside the array it was taken from.
#[kani::proof]
fn subslice_write_inside_allocation() {
    let mut arr = [0u32; 4];
    let s: &mut [u32] = &mut arr[0..2];
    unsafe { *s.get_unchecked_mut(bb(3)) = 7 };
}

#[kani::proof]
fn subslice_read_inside_allocation() {
    let arr = [0u32; 4];
    let s: &[u32] = &arr[0..2];
    let _ = unsafe { *s.get_unchecked(bb(2)) };
}

/// A symbolic index that may be two past the end of the subslice.
#[kani::proof]
fn symbolic_index_past_the_end() {
    let mut arr = [0u8; 8];
    let s: &mut [u8] = &mut arr[2..6];
    let i: usize = kani::any();
    kani::assume(i < 6); // the subslice has 4 elements
    unsafe { *s.get_unchecked_mut(i) = 1 };
}
