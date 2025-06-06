// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem::transmute;

static FOO: Data = Data { a: [0, 1, 0] };

#[derive(Copy, Clone)]
union Data {
    a: [u8; 3],
    b: u16,
}

#[kani::proof]
fn main() {
    let y: u32 = unsafe { transmute(FOO) };
    assert!(y == 256);
}
