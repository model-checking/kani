// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)] // Used for rustc_diagnostic_item.

pub mod invariant;
pub mod slice;

pub use invariant::Invariant;

/// Creates an assumption that will be valid after this statement run. Note that the assumption
/// will only be applied for paths that follow the assumption. If the assumption doesn't hold, the
/// program will exit successfully.
///
/// # Example:
///
/// The code snippet below should never panic.
///
/// ```rust
/// let i : i32 = rmc::any();
/// rmc::assume(i > 10);
/// if i < 0 {
///   panic!("This will never panic");
/// }
/// ```
///
/// The following code may panic though:
///
/// ```rust
/// let i : i32 = rmc::any();
/// assert!(i < 0, "This may panic and verification should fail.");
/// rmc::assume(i > 10);
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "RmcAssume"]
pub fn assume(_cond: bool) {}

/// This creates an symbolic *valid* value of type `T`. You can assign the return value of this
/// function to a variable that you want to make symbolic.
///
/// # Example:
///
/// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
/// under all possible i32 input values.
///
/// ```rust
/// let inputA = rmc::any::<i32>();
/// fn_under_verification(inputA);
/// ```
///
/// Note: This is a safe construct and can only be used with types that implement the `Invariant`
/// trait. The invariant trait is used to constrain the result to ensure the value is valid.
#[inline(always)]
pub fn any<T: Invariant>() -> T {
    let value = unsafe { any_raw::<T>() };
    assume(value.is_valid());
    value
}

/// This function creates an unconstrained value of type `T`. This may result in an invalid value.
///
/// # Example:
///
/// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
/// under all possible values of char, including invalid ones that are greater than char::MAX.
///
/// ```rust
/// let inputA = unsafe { rmc::any_raw::<char>() };
/// fn_under_verification(inputA);
/// ```
#[rustc_diagnostic_item = "RmcAnyRaw"]
#[inline(never)]
pub unsafe fn any_raw<T>() -> T {
    unimplemented!("RMC any_raw")
}

/// This function has been split into a safe and unsafe functions: `rmc::any` and `rmc::any_raw`.
#[deprecated]
#[inline(never)]
pub fn nondet<T: Invariant>() -> T {
    any::<T>()
}

/// Function used in tests for cases where the condition is not always true.
#[inline(never)]
#[rustc_diagnostic_item = "RmcExpectFail"]
pub fn expect_fail(_cond: bool, _message: &str) {}

/// RMC proc macros must be in a separate crate
pub use rmc_macros::*;
