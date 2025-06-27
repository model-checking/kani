// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub slices and string slices functions.

/// Check that we can stub str::is_ascii
pub mod str_check {
    pub fn stub_is_ascii_false(_: &str) -> bool {
        false
    }

    #[kani::proof]
    #[kani::stub(str::is_ascii, stub_is_ascii_false)]
    pub fn check_stub_is_ascii() {
        let input = "is_ascii";
        assert!(!input.is_ascii());
    }
}

/// Check that we can stub slices
pub mod slices_check {
    #[derive(kani::Arbitrary, Ord, PartialOrd, Copy, Clone, PartialEq, Eq)]
    pub struct MyStruct(u8, i32);

    pub fn stub_sort_noop<T>(_: &mut [T]) {}

    #[kani::proof]
    #[kani::stub(<[MyStruct]>::sort, stub_sort_noop)]
    pub fn check_stub_sort_noop() {
        let mut input: [MyStruct; 5] = kani::any();
        let copy = input.clone();
        input.sort();
        assert_eq!(input, copy);
    }
}
