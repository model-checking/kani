// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Check that `frem_fast` triggers overflow checks
// kani-verify-fail

#![feature(core_intrinsics)]

fn main() {
    let x: f64 = kani::any();
    let y: f64 = kani::any();
    let _z = unsafe { std::intrinsics::frem_fast(x, y) };
}
