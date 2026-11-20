// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format terse

extern crate core;

#[kani::proof]
fn check_assert_ne() {
    assert_ne!(0u8, 0u8);
}

#[kani::proof]
fn check_debug_assert_eq() {
    debug_assert_eq!(0u8, 1u8);
}

#[kani::proof]
fn check_core_assert_eq() {
    core::assert_eq!(0u8, 1u8);
}

#[kani::proof]
fn check_assert_matches() {
    std::assert_matches!(Some(0u8), Some(1));
}
