// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `transmute` works as expected when turning raw bytes into
// a `u32`.

// This test is a modified version of the example found in
// https://doc.rust-lang.org/std/intrinsics/fn.transmute.html

#[kani::proof]
fn main() {
    let raw_bytes = [0x78, 0x56, 0x34, 0x12];
    let num = unsafe { std::mem::transmute::<[u8; 4], u32>(raw_bytes) };
    assert_eq!(num, 0x12345678);
}
