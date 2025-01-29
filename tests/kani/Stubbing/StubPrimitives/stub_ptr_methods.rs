// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub methods from raw pointer types.

pub fn stub_len_is_10<T>(_: *const [T]) -> usize {
    10
}

pub fn stub_mut_len_is_0<T>(_: *mut [T]) -> usize {
    0
}

#[kani::proof]
#[kani::stub(<*const [u8]>::len, stub_len_is_10)]
#[kani::stub(<*mut [u8]>::len, stub_mut_len_is_0)]
pub fn check_stub_len_raw_ptr() {
    let mut input: [u8; 5] = kani::any();
    let mut_ptr = &mut input as *mut [u8];
    let ptr = &input as *const [u8];
    assert_eq!(mut_ptr.len(), 0);
    assert_eq!(ptr.len(), 10);
}

pub fn stub_is_always_null<T>(_: *const T) -> bool {
    true
}

// Fix-me: Option doesn't seem to work without the fully qualified path.
#[kani::proof]
#[kani::stub(<*const std::option::Option>::is_null, stub_is_always_null)]
pub fn check_stub_is_null() {
    let input: Option<char> = kani::any();
    let ptr = &input as *const Option<char>;
    assert!(unsafe { ptr.is_null() });
}
