// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#![feature(core_intrinsics)]
use std::intrinsics;

// The code below attempts to leave type `&u32` uninitialized, causing the
// intrinsic `assert_uninit_valid` to generate a panic during compilation.
#[kani::proof]
fn main() {
    let _var: () = unsafe {
        intrinsics::assert_uninit_valid::<&u32>();
    };
}
