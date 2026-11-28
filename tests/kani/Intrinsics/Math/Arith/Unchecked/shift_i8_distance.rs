// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that an in-range `i8` distance on a 128-bit value does not trigger the
// "excessive shift distance" check: 128 is not representable in `i8`.
// See https://github.com/model-checking/kani/issues/5082
#![feature(core_intrinsics)]
use std::intrinsics::{unchecked_shl, unchecked_shr};

#[kani::proof]
fn check_shift_u128_by_i8() {
    let distance: i8 = kani::any();
    kani::assume(distance >= 0);
    let shifted = unsafe { unchecked_shl(1u128, distance) };
    assert!(unsafe { unchecked_shr(shifted, distance) } == 1);
}
