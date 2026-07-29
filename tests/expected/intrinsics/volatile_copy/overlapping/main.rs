// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `volatile_copy_nonoverlapping_memory` fails if the `dst`/`src`
// regions overlap.
#![feature(core_intrinsics)]

#[kani::proof]
fn test_volatile_copy_nonoverlapping_memory_with_overlap() {
    let mut arr: [i32; 3] = [0, 1, 0];
    // Both pointers are derived from `as_mut_ptr` so that the overlap
    // precondition is the *only* thing this harness violates: writing through a
    // pointer derived from a shared borrow (`as_ptr`) would be a second,
    // unintended source of undefined behavior.
    let src: *mut i32 = arr.as_mut_ptr();

    unsafe {
        // The call to `volatile_copy_nonoverlapping_memory` is expected to
        // fail because the `src` region and the `dst` region overlap in
        // `arr[1]`
        let dst = src.add(1);
        core::intrinsics::volatile_copy_nonoverlapping_memory(dst, src, 2);
    }
}
