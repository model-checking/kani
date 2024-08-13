// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani playback works with stubs.
#![allow(dead_code)]

fn is_zero(val: u8) -> bool {
    val == 0
}

fn not_zero(val: u8) -> bool {
    val != 0
}

/// Add a harness that will fail due to incorrect stub but the test will succeed.
#[kani::proof]
#[kani::stub(is_zero, not_zero)]
fn check_bad_stub() {
    assert!(is_zero(kani::any()))
}

fn lt_zero(val: i8) -> bool {
    val < 0
}

fn lt_ten(val: i8) -> bool {
    val < 10
}

/// Add a harness that will fail in an equivalent way.
#[kani::proof]
#[kani::stub(lt_zero, lt_ten)]
fn check_lt_0() {
    assert!(lt_zero(kani::any()))
}
