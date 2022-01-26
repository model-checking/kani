// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This test checks for the case of a valid failure despite the existence of a
// reachable unsupported construct
// kani-flags: --output-format regular --no-default-checks

#![feature(asm)]
fn unsupp(x: &mut u8) {
    unsafe {
        std::arch::asm!("nop");
    }
}

fn main() {
    let mut x = 0;
    if kani::any() {
        unsupp(&mut x);
    } else {
        x = 1;
    }
    assert!(x == 0);
}
