// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani detects UB resulting from converting a raw
//! pointer to a reference when the pointer is not properly aligned.

#[repr(align(4))]
#[derive(Clone, Copy)]
struct AlignedI32(i32);

#[kani::proof]
fn check_misaligned_ptr_cast_fail() {
    let data = AlignedI32(42);
    let ptr = &data as *const AlignedI32;

    unsafe {
        let misaligned = ptr.byte_add(1);
        let x = unsafe { &*misaligned };
    }
}
