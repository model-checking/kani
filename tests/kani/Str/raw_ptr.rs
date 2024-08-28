// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Checks that Kani can handle creating pointers for slices from raw parts.
//! This used to trigger an ICE reported in <https://github.com/model-checking/kani/issues/3312>.
#![feature(ptr_metadata)]

#[derive(kani::Arbitrary)]
struct AscII {
    #[safety_constraint(*inner < 128)]
    inner: u8,
}

#[kani::proof]
fn check_from_raw() {
    let ascii: [AscII; 5] = kani::any();
    let slice_ptr: *const [AscII] = &ascii;
    let (ptr, metadata) = slice_ptr.to_raw_parts();
    let str_ptr: *const str = std::ptr::from_raw_parts(ptr, metadata);
    assert!(unsafe { (&*str_ptr).is_ascii() });
}
