// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z unstable-options -Z stubbing --no-assert-overrides --output-format terse
#![feature(panic_internals)]
#![allow(internal_features)]

extern crate core;

use core::fmt::{Arguments, Debug};
use core::panicking::AssertKind;

#[kani::proof]
fn check_assertion_message() {
    assert_eq!(0u8, 1u8, "a literal custom message");
}

#[kani::proof]
fn check_pattern_message() {
    std::assert_matches!(Some(0u8), Some(1), "a literal pattern message");
}

#[kani::proof]
#[kani::stub(core::panicking::assert_failed, user_stub)]
fn check_user_assertion_stub() {
    assert_eq!(0u8, 1u8);
}

fn user_stub<T: Debug + ?Sized, U: Debug + ?Sized>(
    _: AssertKind,
    _: &T,
    _: &U,
    _: Option<Arguments<'_>>,
) -> ! {
    kani::panic("user assertion failure stub ran")
}
