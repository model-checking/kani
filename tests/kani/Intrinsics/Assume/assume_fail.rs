// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `assume` fails if the condition is false (undefined behavior)
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    unsafe { core::intrinsics::assume(false) };
}
