// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check different violations that can be triggered when providing an usize offset
//! with a type that has size > 1.

#![feature(core_intrinsics)]
use std::intrinsics::offset;
use std::ptr::addr_of;

#[kani::proof]
fn check_intrinsic_args() {
    let array = [0u32];
    let delta: usize = kani::any();
    let new = unsafe { offset(addr_of!(array), delta) };
    assert!(delta <= 1, "Expected 0 and 1 to be the only safe values for offset");
    assert_eq!(new, &array, "This should fail for delta `1`");
    assert_ne!(new, &array, "This should fail for delta `0`");
}
