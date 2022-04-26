// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Checks that `ptr_guaranteed_ne` returns true when both pointers
// are different.
#![feature(core_intrinsics)]
use std::intrinsics::ptr_guaranteed_ne;

#[kani::proof]
fn test_ptr_eq(ptr1: *const u8, ptr2: *const u8) {
    kani::assume(ptr1 != ptr2);
    assert!(ptr_guaranteed_ne(ptr1, ptr2));
    kani::expect_fail(ptr1 == ptr2, "Pointers aren't equal");
}
