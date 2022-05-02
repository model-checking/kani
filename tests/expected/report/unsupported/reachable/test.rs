// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(asm)]
fn unsupp(x: &mut u8) {
    unsafe {
        std::arch::asm!("nop");
    }
}

#[kani::proof]
fn main() {
    let mut x = 0;
    unsupp(&mut x);
    assert!(x == 0);
}
