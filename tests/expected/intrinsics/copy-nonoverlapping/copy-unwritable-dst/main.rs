// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `copy_nonoverlapping` fails when `dst` is not valid for writes.
#[kani::proof]
fn test_copy_nonoverlapping_invalid() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // Get an invalid pointer with an out-of-bounds offset
        let dst_invalid = src.add(3) as *mut i32;
        core::intrinsics::copy_nonoverlapping(src, dst_invalid, 1);
    }
}
