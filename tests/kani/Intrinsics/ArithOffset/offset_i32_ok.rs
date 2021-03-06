// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `arith_offset` returns the expected addresses
#![feature(core_intrinsics)]
use std::intrinsics::arith_offset;

#[kani::proof]
fn test_arith_offset() {
    let arr: [i32; 3] = [1, 2, 3];
    let ptr: *const i32 = arr.as_ptr();

    unsafe {
        assert_eq!(*arith_offset(ptr, 0), 1);
        assert_eq!(*arith_offset(ptr, 1), 2);
        assert_eq!(*arith_offset(ptr, 2), 3);
        assert_eq!(*arith_offset(ptr, 2).sub(1), 2);

        // This wouldn't be okay because it's
        // more than one byte past the object
        // let x = *arith_offset(ptr, 3);

        // Check that the results are the same with a pointer
        // that goes 1 element behind the original one
        let other_ptr: *const i32 = ptr.add(1);

        assert_eq!(*arith_offset(other_ptr, 0), 2);
        assert_eq!(*arith_offset(other_ptr, 1), 3);
        assert_eq!(*arith_offset(other_ptr, 1).sub(1), 2);
    }
}
