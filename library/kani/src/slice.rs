// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::{any, assume, Arbitrary};
use std::alloc::{alloc, dealloc, Layout};
use std::ops::{Deref, DerefMut};

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
/// let slice: kani::slice::AnySlice<u8, 5> = kani::slice::any_slice();
/// foo(&slice); // where foo is a function that takes a slice and verifies a property about it
/// ```
pub struct AnySlice<T, const MAX_SLICE_LENGTH: usize> {
    layout: Layout,
    ptr: *mut T,
    slice_len: usize,
}

impl<T, const MAX_SLICE_LENGTH: usize> AnySlice<T, MAX_SLICE_LENGTH> {
    fn new() -> Self
    where
        T: Arbitrary,
    {
        let any_slice = AnySlice::<T, MAX_SLICE_LENGTH>::alloc_slice();
        unsafe {
            let mut i = 0;
            // Note: even though the guard `i < MAX_SLICE_LENGTH` is redundant
            // since the assumption above guarantees that `slice_len` <=
            // `MAX_SLICE_LENGTH`, without it, CBMC fails to infer the required
            // unwind value, and requires specifying one, which is inconvenient.
            // CBMC also fails to infer the unwinding if the loop is simply
            // written as:
            //     for i in 0..slice_len {
            //         *(ptr as *mut T).add(i) = any();
            //     }
            while i < any_slice.slice_len && i < MAX_SLICE_LENGTH {
                std::ptr::write(any_slice.ptr.add(i), any());
                i += 1;
            }
        }
        any_slice
    }

    fn alloc_slice() -> Self {
        let slice_len = any();
        assume(slice_len <= MAX_SLICE_LENGTH);
        let layout = Layout::array::<T>(slice_len).unwrap();
        let ptr = if slice_len == 0 { std::ptr::null() } else { unsafe { alloc(layout) } };
        Self { layout, ptr: ptr as *mut T, slice_len }
    }

    pub fn get_slice(&self) -> &[T] {
        if self.slice_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.slice_len) }
        }
    }

    pub fn get_slice_mut(&mut self) -> &mut [T] {
        if self.slice_len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr, self.slice_len) }
        }
    }
}

impl<T, const MAX_SLICE_LENGTH: usize> Drop for AnySlice<T, MAX_SLICE_LENGTH> {
    fn drop(&mut self) {
        if self.slice_len > 0 {
            assert!(!self.ptr.is_null());
            unsafe {
                dealloc(self.ptr as *mut u8, self.layout);
            }
        }
    }
}

impl<T, const MAX_SLICE_LENGTH: usize> Deref for AnySlice<T, MAX_SLICE_LENGTH> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.get_slice()
    }
}

impl<T, const MAX_SLICE_LENGTH: usize> DerefMut for AnySlice<T, MAX_SLICE_LENGTH> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_slice_mut()
    }
}

pub fn any_slice<T, const MAX_SLICE_LENGTH: usize>() -> AnySlice<T, MAX_SLICE_LENGTH>
where
    T: Arbitrary,
{
    AnySlice::<T, MAX_SLICE_LENGTH>::new()
}
