// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-verify-fail

#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: i8 = -128;
    let b: i8 = -1;
    let i = unsafe { std::intrinsics::exact_div(a, b) };
}
