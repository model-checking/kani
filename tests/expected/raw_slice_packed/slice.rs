// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This test is basically the same as ../raw_slice/slice.rs but using repr(C) instead.

//! This test case has a bunch of checks related to structures using raw slices ([T]).
use std::mem;

/// Non-sized structure with a sized element and an unsized element.
#[repr(packed)]
struct NonEmptySlice {
    first: u8,
    others: [u8],
}

/// Non-sized structure with only an unsized element.
#[repr(packed)]
struct RawSlice {
    inner: [u8],
}

impl NonEmptySlice {
    /// This creates a NonEmptySlice from a byte slice.
    ///
    /// Note that the cast operation keep the the fat pointer structure, so we need to manually
    /// create the expected pointer with correct len. For the NonEmptySlice, this looks like:
    /// ```rust
    /// struct NonEmptySliceRef {
    ///   ptr: *const u8,
    ///   unsize_len: usize,
    /// }
    /// ```
    ///
    /// I.e.: The length is only relative to the unsized part of the structure.
    /// The total size of the object is the `size_of(sized_members) + (unsize_len * size_of::<u8>())`
    ///
    fn new(bytes: &mut [u8]) -> &Self {
        assert!(bytes.len() > 0, "This requires at least one element");
        let unsized_len = bytes.len() - 1;
        unsafe {
            let ptr = std::ptr::slice_from_raw_parts_mut(bytes.as_mut_ptr(), unsized_len);
            unsafe { &*(ptr as *mut Self) }
        }
    }

    /// This function does a naive transmute in the slice which generates a corrupt structure.
    /// See the documentation of `NonEmptySliceRef::new()` for more details.
    fn naive_new(bytes: &mut [u8]) -> &Self {
        assert!(bytes.len() > 0, "This requires at least one element");
        unsafe { std::mem::transmute(bytes) }
    }
}

impl RawSlice {
    fn new(bytes: &mut [u8]) -> &Self {
        unsafe { std::mem::transmute(bytes) }
    }
}

#[kani::proof]
fn check_non_empty_raw() {
    let mut vector = vec![1u8, 2u8, 3u8, 4u8];
    let raw = RawSlice::new(vector.as_mut_slice());
    assert_eq!(mem::size_of_val(raw), 4);
    assert_eq!(raw.inner.len(), 4);
    assert_eq!(raw.inner[0], 1);
}

#[kani::proof]
fn check_empty_raw() {
    let mut vector = vec![];
    let raw = RawSlice::new(vector.as_mut_slice());
    assert_eq!(mem::size_of_val(raw), 0);
    assert_eq!(raw.inner.len(), 0);
}

#[kani::proof]
fn check_non_empty_slice() {
    let mut vector = vec![1u8, 5u8];
    let slice = NonEmptySlice::new(vector.as_mut_slice());
    assert_eq!(mem::size_of_val(slice), 2);
    assert_eq!(slice.others.len(), 1);
    assert_eq!(slice.first, 1);
    assert_eq!(slice.others[0], 5);
}

#[kani::proof]
#[kani::unwind(3)]
fn check_naive_iterator_should_fail() {
    let mut bytes = vec![1u8, 5u8];
    assert_eq!(bytes.len(), 2);

    let first = bytes[0];
    let second = bytes[1];

    let slice = NonEmptySlice::naive_new(&mut bytes);
    assert_eq!(slice.others.len(), 2, "Naive new should have the wrong slice len");
    assert_eq!(slice.first, first);
    assert_eq!(slice.others[0], second);
    let mut sum = 0u8;
    for e in &slice.others {
        // This should trigger out-of-bounds.
        sum = sum.wrapping_add(*e);
    }
}
