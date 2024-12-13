// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `copy` fails when `src` is not valid for reads.
#[kani::proof]
fn test_copy_invalid() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // Get an invalid pointer with a negative offset
        let src_invalid = unsafe { src.sub(1) as *const i32 };
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy(src_invalid, dst, 1);
    }
}
