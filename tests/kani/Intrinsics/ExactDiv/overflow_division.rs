// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `exact_div` results in undefined behavior if `x == T::MIN && y == -1`
// https://doc.rust-lang.org/std/intrinsics/fn.exact_div.html
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x = i32::MIN;
    let y = -1;
    let _ = unsafe { std::intrinsics::exact_div(x, y) };
}
