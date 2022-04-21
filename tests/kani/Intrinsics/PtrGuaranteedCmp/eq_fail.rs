// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Checks that `ptr_guaranteed_eq` returns a nondet. value when both pointers
// are equal, which causes the test to fail in the `else` branch.
#![feature(core_intrinsics)]
use std::intrinsics::ptr_guaranteed_eq;

#[kani::proof]
fn test_ptr_eq(ptr1: *const u8, ptr2: *const u8) {
    kani::assume(ptr1 == ptr2);
    if ptr_guaranteed_eq(ptr1, ptr2) {
        assert!(ptr1 == ptr2);
    } else {
        assert!(ptr1 != ptr2);
    }
}
