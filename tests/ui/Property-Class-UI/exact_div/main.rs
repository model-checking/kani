// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: u8 = 8;
    let b: u8 = 4;
    let i = unsafe { std::intrinsics::exact_div(a, b) };
    assert!(i == 2);
}
