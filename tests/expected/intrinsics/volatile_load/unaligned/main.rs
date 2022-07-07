// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `volatile_load` fails when `src` is not aligned.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let vec = vec![0u32; 2];
    let vec_ptr = vec.as_ptr();
    unsafe {
        // Obtain an unaligned pointer by casting into `*const u8`,
        // adding an offset of 1 and casting back into `*const u32`
        let vec_ptr_u8: *const u8 = vec_ptr as *const u8;
        let unaligned_ptr = vec_ptr_u8.add(1) as *const u32;
        let _value = std::intrinsics::volatile_load(unaligned_ptr);
    }
}
