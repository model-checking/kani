// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::{any, assume};

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
