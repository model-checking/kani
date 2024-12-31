// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check different violations that can be triggered when providing an usize offset
//! with a type that has size > 1.

#![feature(core_intrinsics)]
use std::intrinsics::offset;
use std::ptr::addr_of;

/// This harness exercises different scenarios when providing unconstrained offset counter.
///
/// We expect the following UB to be detected:
/// 1. The offset value, `delta`, itself is greater than `isize::MAX`.
/// 2. The offset in bytes, `delta * size_of::<u32>()`,  is greater than `isize::MAX`.
/// 3. Offset result does not point to the same allocation as the original pointer.
///
/// The offset operation should only succeed for delta values:
/// - `0`: The new pointer is the same as the base of the array.
/// - `1`: The new pointer points to the end of the allocation.
///
/// FIXME: Because of CBMC wrapping behavior with pointer arithmetic, the assertion that checks
/// that `delta <= 1` currently fails. See <https://github.com/model-checking/kani/issues/1150>.
#[kani::proof]
fn check_intrinsic_args() {
    let array = [0u32];
    let delta: usize = kani::any();
    let new = unsafe { offset(addr_of!(array), delta) };
    assert!(delta <= 1, "Expected 0 and 1 to be the only safe values for offset");
    assert_eq!(new, &array, "This should fail for delta `1`");
    assert_ne!(new, &array, "This should fail for delta `0`");
}
