// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `copy_nonoverlapping` fails if the `src`/`dst` regions overlap.
#[kani::proof]
fn test_copy_nonoverlapping_with_overlap() {
    let arr: [i32; 3] = [0, 1, 0];
    let src: *const i32 = arr.as_ptr();

    unsafe {
        // The call to `copy_nonoverlapping` is expected to fail because
        // the `src` region and the `dst` region overlap in `arr[1]`
        let dst = src.add(1) as *mut i32;
        core::intrinsics::copy_nonoverlapping(src, dst, 2);
    }
}
