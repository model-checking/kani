// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#![feature(core_intrinsics)]

// Ungracefully aborts the execution of the process, like `abort`.
//
// This is the intrinsic behind `core::process::abort_immediate`. Kani models
// reaching it as a failed assertion, matching the handling of `abort`.
#[kani::proof]
fn main() {
    core::intrinsics::abort_immediate();
}
