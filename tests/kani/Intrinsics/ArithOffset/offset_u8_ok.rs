// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `arith_offset` returns the expected addresses
#![feature(core_intrinsics)]
use std::intrinsics::arith_offset;

#[kani::proof]
fn test_arith_offset() {
    let s: &str = "123";
    let ptr: *const u8 = s.as_ptr();

    unsafe {
        assert_eq!(*arith_offset(ptr, 0) as char, '1');
        assert_eq!(*arith_offset(ptr, 1) as char, '2');
        assert_eq!(*arith_offset(ptr, 2) as char, '3');
        assert_eq!(*arith_offset(ptr, 2).sub(1) as char, '2');

        // This is okay because it's one byte past the object,
        // but dereferencing it is UB
        let _x = arith_offset(ptr, 3);

        // Check that the results are the same with a pointer
        // that goes 1 element behind the original one
        let other_ptr: *const u8 = ptr.add(1);

        assert_eq!(*arith_offset(other_ptr, 0) as char, '2');
        assert_eq!(*arith_offset(other_ptr, 1) as char, '3');
        assert_eq!(*arith_offset(other_ptr, 1).sub(1) as char, '2');
    }
}
