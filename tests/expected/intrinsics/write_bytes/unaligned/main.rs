// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `write_bytes` fails when `dst` is not aligned.

// This test is a modified version of the example in
// https://doc.rust-lang.org/std/ptr/fn.write_bytes.html
use std::intrinsics::write_bytes;

#[kani::proof]
fn main() {
    let mut vec = vec![0u32; 4];
    unsafe {
        let vec_ptr = vec.as_mut_ptr();
        // Obtain an unaligned pointer by casting into `*mut u8`,
        // adding an offset of 1 and casting back into `*mut u32`
        let vec_ptr_u8: *mut u8 = vec_ptr as *mut u8;
        let unaligned_ptr = vec_ptr_u8.add(1) as *mut u32;
        write_bytes(unaligned_ptr, 0xfe, 2);
    }
    assert_eq!(vec, [0xfefefe00, 0xfefefefe, 0x000000fe, 0]);
}
