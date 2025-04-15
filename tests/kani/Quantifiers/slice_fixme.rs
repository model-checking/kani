// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
    let bytes = kani::slice::any_slice_of_array(&arr);
    let ptr = bytes.as_ptr();
    kani::assume(bytes.len() > 0);
    unsafe {
        kani::assume(
            kani::forall!(|i in (0,bytes.len())| *ptr.wrapping_byte_offset(i as isize) < 8),
        );
    }
    kani::assert(bytes[0] < 8, "");
}
