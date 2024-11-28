// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani emits an error when
//! `kani::float::float_to_int_in_range` is instantiated with a non-float type

#[kani::proof]
fn check_invalid_float() {
    let i: i32 = 5;
    let _c = kani::float::float_to_int_in_range::<i32, u8>(i);
}
