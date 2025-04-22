// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! FIXME: <https://github.com/model-checking/kani/issues/4019>

extern crate kani;
use kani::kani_forall;

#[kani::proof]
fn vec_assert_forall_harness() {
    let v = vec![10 as u8; 128];
    let ptr = v.as_ptr();
    unsafe {
        kani::assert(kani::forall!(|i in (0,128)| *ptr.wrapping_byte_offset(i as isize) == 10), "");
    }
}

#[kani::proof]
fn slice_assume_forall_harness() {
    let arr: [u8; 8] = kani::any();
    let ptr = arr.as_ptr();
    unsafe {
        kani::assume(kani::forall!(|i in (0,arr.len())| *ptr.wrapping_byte_offset(i as isize) < 8));
    }
    kani::assert(arr[0] < 8, "");
}

#[kani::proof]
fn slice_assume_sorted_harness() {
    let arr: [u8; 12] = kani::any();
    let ptr = arr.as_ptr();
    unsafe {
        kani::assume(
            kani::forall!(|i in (0,arr.len()-1)| *ptr.wrapping_byte_offset(i as isize) < *ptr.wrapping_byte_offset((i+1) as isize)),
        );
    }
    kani::assert(arr[0] < arr[1], "");
}
