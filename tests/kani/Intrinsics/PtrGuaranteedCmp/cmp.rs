// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `ptr_guaranteed_eq` returns true if the pointers are equal, false
// otherwise.
#![feature(core_intrinsics)]
use std::intrinsics::ptr_guaranteed_cmp;

fn ptr_eq(ptr1: *const u8, ptr2: *const u8) -> bool {
    ptr_guaranteed_cmp(ptr1, ptr2) == 1
}

fn ptr_ne(ptr1: *const u8, ptr2: *const u8) -> bool {
    ptr_guaranteed_cmp(ptr1, ptr2) == 0
}

#[kani::proof]
fn check_ptr_guaranteed_cmp() {
    let v1: u8 = kani::any();
    let v2: u8 = kani::any();
    assert!(ptr_eq(&v1, &v1));
    assert!(ptr_eq(&v2, &v2));
    assert!(ptr_ne(&v2, &v1));
    assert!(ptr_ne(&v1, &v2));
}
