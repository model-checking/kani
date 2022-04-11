// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `exact_div` returns the expected result if none
// of the conditions for undefined behavior are met
// https://doc.rust-lang.org/std/intrinsics/fn.exact_div.html
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x = 8;
    let y = 4;
    let res = unsafe { std::intrinsics::exact_div(x, y) };
    assert!(res == 2);
}
