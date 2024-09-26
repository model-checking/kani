// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z mem-predicates
//! Check the behavior of the new `PointerGenerator`.
extern crate kani;

use kani::{cover, AllocationStatus, PointerGenerator};

/// Harness that checks that all cases are covered and the code behaves as expected.
///
/// Note that for `DeadObject`, `Dangling`, and `OutOfBounds` the predicate will fail due to demonic non-determinism.
#[kani::proof]
fn check_arbitrary_ptr() {
    let mut generator = PointerGenerator::<char, 3>::new();
    let arbitrary = generator.any_alloc_status();
    let ptr = arbitrary.ptr;
    match arbitrary.status {
        AllocationStatus::Dangling => {
            cover!(true, "Dangling");
            assert!(!kani::mem::can_write_unaligned(ptr), "Dangling write");
        }
        AllocationStatus::Null => {
            assert!(!kani::mem::can_write_unaligned(ptr), "NullPtr");
        }
        AllocationStatus::DeadObject => {
            // Due to demonic non-determinism, the API will trigger an error.
            assert!(!kani::mem::can_write_unaligned(ptr), "DeadObject");
        }
        AllocationStatus::OutOfBounds => {
            assert!(!kani::mem::can_write_unaligned(ptr), "OutOfBounds");
        }
        AllocationStatus::InBounds => {
            // This should always succeed
            assert!(kani::mem::can_write_unaligned(ptr), "InBounds");
        }
    };
}
