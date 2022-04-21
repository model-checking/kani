// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces stubs for process methods.

// Export everything else from std::process.
pub use std::process::*;

#[inline(always)]
pub fn abort() -> ! {
    panic!("Function abort() was invoked")
}

#[inline(always)]
pub fn exit(_code: i32) -> ! {
    panic!("Function exit() was invoked")
}
