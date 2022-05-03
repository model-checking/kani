// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `write_bytes` works as expected.

// This test is a modified version of the example in
// https://doc.rust-lang.org/std/ptr/fn.write_bytes.html
#![feature(core_intrinsics)]
use std::intrinsics::write_bytes;

#[kani::proof]
fn main() {
    let mut vec = vec![0u32; 4];
    unsafe {
        let vec_ptr = vec.as_mut_ptr();
        write_bytes(vec_ptr, 0xfe, 2);
    }
    assert_eq!(vec, [0xfefefefe, 0xfefefefe, 0, 0]);
}
