// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#![feature(core_intrinsics)]
use std::intrinsics;

// The code below attempts to zero-initialize type `&i32`, causing the intrinsic
// `assert_zero_valid` to generate a panic during compilation.
#[kani::proof]
fn main() {
    let _var: () = unsafe {
        intrinsics::assert_zero_valid::<&i32>();
    };
}
