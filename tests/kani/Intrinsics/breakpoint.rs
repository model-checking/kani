// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `breakpoint` is supported (generates a `SKIP` statement)
// and that it does not affect the verification result
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    unsafe { std::intrinsics::breakpoint() };
    assert!(true);
}
