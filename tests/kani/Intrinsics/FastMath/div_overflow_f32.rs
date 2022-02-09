// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `fdiv_fast` triggers overflow checks
// kani-verify-fail

#![feature(core_intrinsics)]

fn main() {
    let x: f32 = kani::any();
    let y: f32 = kani::any();
    let _z = unsafe { std::intrinsics::fdiv_fast(x, y) };
}
