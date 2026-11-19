// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z unstable-options --no-assert-overrides --output-format terse
// compile-flags: -Cllvm-args=-Zstubbing
//! Allow the stub annotation without enabling the compiler's stub pass.

#![feature(panic_internals)]
#![allow(internal_features)]

extern crate core;

use core::fmt::{Arguments, Debug};

#[kani::proof]
#[kani::stub(core::panicking::assert_matches_failed, pattern_stub)]
fn check_inactive_pattern_stub() {
    std::assert_matches!(Some(0u8), Some(1));
}

fn pattern_stub<T: Debug + ?Sized>(_: &T, _: &str, _: Option<Arguments<'_>>) -> ! {
    kani::panic("inactive user stub ran")
}
