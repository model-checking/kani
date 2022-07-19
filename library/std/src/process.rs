// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces stubs for process methods.

// Export everything else from std::process.
pub use std::process::*;

#[inline(always)]
pub fn abort() -> ! {
    kani::panic("Function abort() was invoked")
}

#[inline(always)]
pub fn exit(_code: i32) -> ! {
    kani::panic("Function exit() was invoked")
}
