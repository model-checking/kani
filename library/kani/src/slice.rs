// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::{any, any_raw, assume};
use core::ops::{Deref, DerefMut};
use std::{
    alloc::{alloc, dealloc, Layout},
    marker::PhantomData,
};

/// Given an array `arr` of length `LENGTH`, this function returns a **valid**
/// slice of `arr` with non-deterministic start and end points.  This is useful
/// in situations where one wants to verify that all possible slices of a given
/// array satisfy some property.
///
/// # Example:
///
/// ```rust
/// let arr = [1, 2, 3];
/// let slice = kani::slice::any_slice_of_array(&arr);
/// foo(slice); // where foo is a function that takes a slice and verifies a property about it
/// ```
pub fn any_slice_of_array<T, const LENGTH: usize>(arr: &[T; LENGTH]) -> &[T] {
    let (from, to) = any_range::<LENGTH>();
    &arr[from..to]
}

/// A mutable version of the previous function
pub fn any_slice_of_array_mut<T, const LENGTH: usize>(arr: &mut [T; LENGTH]) -> &mut [T] {
    let (from, to) = any_range::<LENGTH>();
    &mut arr[from..to]
}

fn any_range<const LENGTH: usize>() -> (usize, usize) {
    let from: usize = any();
    let to: usize = any();
    assume(to <= LENGTH);
    assume(from <= to);
    (from, to)
}

/// A struct that stores a slice of type `T` with a non-deterministic length
/// between `0..=MAX_SLICE_LENGTH` and with non-deterministic content.  This is
/// useful in situations where one wants to verify that all slices with any
/// content and with a length up to `MAX_SLICE_LENGTH` satisfy a certain
/// property. Use `any_slice` to create an instance of this struct.
///
/// # Example:
///
/// ```rust
/// let slice: kani::slice::NonDetSlice<u8, 5> = kani::slice::any_slice();
/// foo(&slice); // where foo is a function that takes a slice and verifies a property about it
/// ```
pub struct NonDetSlice<T, const MAX_SLICE_LENGTH: usize> {
    layout: Layout,
    ptr: *mut u8,
    slice_len: usize,
    _phantom: PhantomData<T>,
}

impl<T, const MAX_SLICE_LENGTH: usize> NonDetSlice<T, MAX_SLICE_LENGTH> {
    fn new() -> Self {
        let slice_len: usize = any();
        assume(slice_len <= MAX_SLICE_LENGTH);
        let layout = Layout::array::<T>(slice_len).unwrap();
        let ptr = unsafe { alloc(layout) };
        Self { layout, ptr, slice_len, _phantom: PhantomData }
    }

    pub fn get_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr as *mut T, self.slice_len) }
    }

    pub fn get_slice_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut T, self.slice_len) }
    }
}

impl<T, const MAX_SLICE_LENGTH: usize> Drop for NonDetSlice<T, MAX_SLICE_LENGTH> {
    fn drop(&mut self) {
        if self.slice_len > 0 {
            unsafe {
                dealloc(self.ptr, self.layout);
            }
        }
    }
}

impl<T, const MAX_SLICE_LENGTH: usize> Deref for NonDetSlice<T, MAX_SLICE_LENGTH> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.get_slice()
    }
}

impl<T, const MAX_SLICE_LENGTH: usize> DerefMut for NonDetSlice<T, MAX_SLICE_LENGTH> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_slice_mut()
    }
}

pub fn any_slice<T, const MAX_SLICE_LENGTH: usize>() -> NonDetSlice<T, MAX_SLICE_LENGTH> {
    NonDetSlice::<T, MAX_SLICE_LENGTH>::new()
}
