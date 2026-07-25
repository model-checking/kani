// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `volatile_copy_nonoverlapping_memory` fails if the `dst`/`src`
// regions overlap.
#![feature(core_intrinsics)]

#[kani::proof]
fn test_volatile_copy_nonoverlapping_memory_with_overlap() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // The call to `volatile_copy_nonoverlapping_memory` is expected to
        // fail because the `src` region and the `dst` region overlap in
        // `arr[1]`
        let dst = src.add(1) as *mut i32;
        core::intrinsics::volatile_copy_nonoverlapping_memory(dst, src, 2);
    }
}
