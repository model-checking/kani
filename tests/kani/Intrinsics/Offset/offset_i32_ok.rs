// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `offset` returns the expected addresses
#![feature(core_intrinsics)]
use std::intrinsics::offset;

#[kani::proof]
fn test_offset() {
    let arr: [i32; 3] = [1, 2, 3];
    let ptr: *const i32 = arr.as_ptr();

    unsafe {
        assert_eq!(*offset(ptr, 0isize), 1);
        assert_eq!(*offset(ptr, 1isize), 2);
        assert_eq!(*offset(ptr, 2isize), 3);
        assert_eq!(*offset(ptr, 2isize).sub(1), 2);

        // This wouldn't be okay because it's
        // more than one byte past the object
        // let x = *offset(ptr, 3);

        // Check that the results are the same with a pointer
        // that goes 1 element behind the original one
        let other_ptr: *const i32 = ptr.add(1);

        assert_eq!(*offset(other_ptr, 0isize), 2);
        assert_eq!(*offset(other_ptr, 1isize), 3);
        assert_eq!(*offset(other_ptr, 1isize).sub(1), 2);
    }
}
