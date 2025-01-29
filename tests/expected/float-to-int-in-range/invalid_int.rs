// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani emits an error when
//! `kani::float::float_to_int_in_range` is instantiated with a non-integer type

#[kani::proof]
fn check_invalid_integer() {
    let f: f32 = kani::any();
    let _c = kani::float::float_to_int_in_range::<f32, bool>(f);
}
