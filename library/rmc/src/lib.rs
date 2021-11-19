// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)] // Used for rustc_diagnostic_item.

pub mod slice;

/// Creates an assumption that will be valid after this statement run. Note that the assumption
/// will only be applied for paths that follow the assumption. If the assumption doesn't hold, the
/// program will exit successfully.
///
/// # Example:
///
/// The code snippet below should never panic.
///
/// ```rust
/// let i : i32 = unsafe { rmc::nondet() };
/// rmc::assume(i > 10);
/// if i < 0 {
///   panic!("This will never panic");
/// }
/// ```
///
/// The following code may panic though:
///
/// ```rust
/// let i : i32 = unsafe { rmc::nondet() };
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
/// let inputA = unsafe { rmc::nondet::<i32>() };
/// fn_under_verification(inputA);
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "RmcNonDet"]
pub unsafe fn nondet<T>() -> T {
    unimplemented!("RMC nondet")
}

/// Function used in tests for cases where the condition is not always true.
#[inline(never)]
#[rustc_diagnostic_item = "RmcExpectFail"]
pub fn expect_fail(_cond: bool, _message: &str) {}

/// RMC proc macros must be in a separate crate
pub use rmc_macros::*;
