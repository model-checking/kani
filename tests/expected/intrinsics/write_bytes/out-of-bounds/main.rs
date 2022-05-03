// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `write_bytes` fails if an out-of-bounds write is made.

// This test is a modified version of the example in
// https://doc.rust-lang.org/std/ptr/fn.write_bytes.html
#![feature(core_intrinsics)]
use std::intrinsics::write_bytes;

#[kani::proof]
fn main() {
    let mut vec = vec![0u32; 4];
    unsafe {
        let vec_ptr = vec.as_mut_ptr().add(4);
        write_bytes(vec_ptr, 0xfe, 1);
    }
    assert_eq!(vec, [0, 0, 0, 0]);
}
