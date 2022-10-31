// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `ptr_guaranteed_eq` returns true if the pointers are equal, false
// otherwise.
#![feature(core_intrinsics)]
use std::intrinsics::ptr_guaranteed_cmp;

#[kani::proof]
fn test_ptr_eq(ptr1: *const u8, ptr2: *const u8) {
    kani::assume(ptr1 == ptr2);
    assert_eq!(ptr_guaranteed_cmp(ptr1, ptr2), 1);
}

#[kani::proof]
fn test_ptr_ne(ptr1: *const u8, ptr2: *const u8) {
    kani::assume(ptr1 != ptr2);
    assert_eq!(ptr_guaranteed_cmp(ptr1, ptr2), 0);
}
