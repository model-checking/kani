// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `exact_div` results in undefined behavior if `y == 0`
// https://doc.rust-lang.org/std/intrinsics/fn.exact_div.html
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x = kani::any();
    let y = 0;
    let _ = unsafe { std::intrinsics::exact_div(x, y) };
}
