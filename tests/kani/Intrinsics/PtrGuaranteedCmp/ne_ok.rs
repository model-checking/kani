// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `ptr_guaranteed_ne` returns false when the pointers are
// equal, which causes the test to pass in the `else` branch. The `if`
// branch is unreachable.
#![feature(core_intrinsics)]
use std::intrinsics::ptr_guaranteed_ne;

#[kani::proof]
fn test_ptr_ne(ptr1: *const u8, ptr2: *const u8) {
    kani::assume(ptr1 == ptr2);
    if ptr_guaranteed_ne(ptr1, ptr2) {
        assert!(ptr1 != ptr2);
    } else {
        assert!(ptr1 == ptr2);
    }
}
