// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::mem::{self, MaybeUninit};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct Demo(bool, u16);

#[kani::proof]
fn main() {
    unsafe {
        // Transmute-round-trip through a type with Scalar layout is lossless.
        // This is tricky because that 'scalar' is *partially* uninitialized.
        let x = Demo(true, 3);
        let y: MaybeUninit<u32> = mem::transmute(x);
        assert_eq!(x, mem::transmute(y));
    }
}
