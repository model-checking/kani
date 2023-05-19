// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `offset` returns the expected addresses
#![feature(core_intrinsics)]
use std::intrinsics::offset;

#[kani::proof]
fn test_offset() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        assert_eq!(*offset(ptr, 0isize) as char, '1');
        assert_eq!(*offset(ptr, 1isize) as char, '2');
        assert_eq!(*offset(ptr, 2isize) as char, '3');
        assert_eq!(*offset(ptr, 2isize).sub(1) as char, '2');

        // This is okay because it's one byte past the object,
        // but dereferencing it is UB
        let _x = offset(ptr, 3isize);

        // Check that the results are the same with a pointer
        // that goes 1 element behind the original one
        let other_ptr: *const u8 = ptr.add(1);

        assert_eq!(*offset(other_ptr, 0isize) as char, '2');
        assert_eq!(*offset(other_ptr, 1isize) as char, '3');
        assert_eq!(*offset(other_ptr, 1isize).sub(1) as char, '2');
    }
}
