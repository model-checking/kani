// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `volatile_set_memory` fills `count` elements starting at `dst`
// with the byte value `val`.
#![feature(core_intrinsics)]

#[kani::proof]
fn test_volatile_set_memory_simple() {
    let mut arr: [u8; 4] = [0, 0, 0, 0];
    let dst: *mut u8 = arr.as_mut_ptr();
    unsafe {
        core::intrinsics::volatile_set_memory(dst, 0xAB, 4);
    }
    assert!(arr == [0xAB, 0xAB, 0xAB, 0xAB]);
}

#[kani::proof]
fn test_volatile_set_memory_partial() {
    let mut arr: [u8; 4] = [1, 2, 3, 4];
    unsafe {
        // Only fill the first two elements.
        let dst: *mut u8 = arr.as_mut_ptr();
        core::intrinsics::volatile_set_memory(dst, 0, 2);
    }
    assert!(arr == [0, 0, 3, 4]);
}
