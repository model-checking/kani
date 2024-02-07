// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z mem-predicates
//! Check that Kani's memory predicates work for fat pointers.

extern crate kani;

use kani::mem::assert_valid_ptr;
use std::fmt::Debug;

mod valid_access {
    use super::*;
    #[kani::proof]
    pub fn check_valid_dyn_ptr() {
        let boxed: Box<dyn PartialEq<u8>> = Box::new(10u8);
        let raw_ptr = Box::into_raw(boxed);
        assert_valid_ptr(raw_ptr);
        let boxed = unsafe { Box::from_raw(raw_ptr) };
        assert_ne!(*boxed, 0);
    }

    #[kani::proof]
    pub fn check_valid_slice_ptr() {
        let array = ['a', 'b', 'c'];
        let slice = &array as *const [char];
        assert_valid_ptr(slice);
        assert_eq!(unsafe { &*slice }[0], 'a');
        assert_eq!(unsafe { &*slice }[2], 'c');
    }
}

mod invalid_access {
    use super::*;
    #[kani::proof]
    #[kani::should_panic]
    pub fn check_invalid_dyn_ptr() {
        let raw_ptr: *const dyn PartialEq<u8> = unsafe { new_dead_ptr::<u8>() };
        assert_valid_ptr(raw_ptr);
        assert_eq!(*unsafe { &*raw_ptr }, 0u8);
    }

    unsafe fn new_dead_ptr<T: Default>() -> *const T {
        let var = T::default();
        &var as *const _
    }
}
