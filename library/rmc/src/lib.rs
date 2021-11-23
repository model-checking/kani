// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)] // Used for rustc_diagnostic_item.

use core::ops::{Deref, DerefMut};

/// Creates an assumption that will be valid after this statement run. Note that the assumption
/// will only be applied for paths that follow the assumption. If the assumption doesn't hold, the
/// program will exit successfully.
///
/// # Example:
///
/// The code snippet below should never panic.
///
/// ```rust
/// let i : i32 = rmc::nondet();
/// rmc::assume(i > 10);
/// if i < 0 {
///   panic!("This will never panic");
/// }
/// ```
///
/// The following code may panic though:
///
/// ```rust
/// let i : i32 = rmc::nondet();
/// assert!(i < 0, "This may panic and verification should fail.");
/// rmc::assume(i > 10);
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "RmcAssume"]
pub fn assume(_cond: bool) {}

/// This creates an unconstrained value of type `T`. You can assign the return value of this
/// function to a variable that you want to make symbolic.
///
/// # Example:
///
/// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
/// under all possible i32 input values.
///
/// ```rust
/// let inputA = rmc::nondet::<i32>();
/// fn_under_verification(inputA);
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "RmcNonDet"]
pub fn nondet<T>() -> T {
    unimplemented!("RMC nondet")
}

/// Function used in tests for cases where the condition is not always true.
#[inline(never)]
#[rustc_diagnostic_item = "RmcExpectFail"]
pub fn expect_fail(_cond: bool, _message: &str) {}

/// Given an array `arr` of length `LENGTH`, this function returns a **valid**
/// slice of `arr` with non-deterministic start and end points.  This is useful
/// in situations where one wants to verify that all possible slices of a given
/// array satisfy some property.
///
/// # Example:
///
/// ```rust
/// let arr = [1, 2, 3];
/// let slice = rmc::nondet_slice(&arr);
/// foo(slice); // where foo is a function that takes a slice and verifies a property about it
/// ```
pub fn nondet_slice<T, const LENGTH: usize>(arr: &[T; LENGTH]) -> &[T] {
    let (from, to) = nondet_range::<LENGTH>();
    &arr[from..to]
}

/// A mutable version of the previous function
pub fn nondet_slice_mut<T, const LENGTH: usize>(arr: &mut [T; LENGTH]) -> &mut [T] {
    let (from, to) = nondet_range::<LENGTH>();
    &mut arr[from..to]
}

fn nondet_range<const LENGTH: usize>() -> (usize, usize) {
    let from: usize = nondet();
    let to: usize = nondet();
    assume(to <= LENGTH);
    assume(from <= to);
    (from, to)
}

/// A struct that creates a slice of type `T` with a non-deterministic length
/// between `0..MAX_SLICE_LENGTH` and with non-deterministic content.  This is
/// useful in situations where one wants to verify that all slices with any
/// content and with a length up to `MAX_SLICE_LENGTH` satisfy a certain
/// property
///
/// # Example:
///
/// ```rust
/// let slice = rmc::NonDetSlice::<u8, 5>::new();
/// foo(&slice); // where foo is a function that takes a slice and verifies a property about it
/// ```
pub struct NonDetSlice<T, const MAX_SLICE_LENGTH: usize> {
    arr: [T; MAX_SLICE_LENGTH],
    slice_len: usize,
}

impl<T, const MAX_SLICE_LENGTH: usize> NonDetSlice<T, MAX_SLICE_LENGTH> {
    pub fn new() -> Self {
        let arr: [T; MAX_SLICE_LENGTH] = nondet();
        let slice_len: usize = nondet();
        assume(slice_len <= MAX_SLICE_LENGTH);
        Self { arr, slice_len }
    }

    pub fn get_slice(&self) -> &[T] {
        &self.arr[..self.slice_len]
    }

    pub fn get_slice_mut(&mut self) -> &mut [T] {
        &mut self.arr[..self.slice_len]
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
