// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(asm)]

#[kani::proof]
fn main() {
    unsafe {
        asm!("nop");
    }
}
