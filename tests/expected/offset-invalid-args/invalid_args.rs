// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check Kani output if the arguments provided to the offset intrisic are incorrect.

#![feature(core_intrinsics)]
use std::intrinsics::offset;

/// The rust compiler currently ICE.
#[kani::proof]
fn check_intrinsic_args() {
    let array = [0];
    let delta: u32 = kani::any();
    let new = unsafe { offset(&array, delta) };
    assert_ne!(new, &array)
}
