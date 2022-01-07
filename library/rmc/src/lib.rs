// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)] // Used for rustc_diagnostic_item.

pub mod arbitrary;
pub mod invariant;
pub mod slice;

pub use arbitrary::Arbitrary;
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
/// under all possible `NonZeroU8` input values, i.e., all possible `u8` values except zero.
///
/// ```rust
/// let inputA = rmc::any::<std::num::NonZeroU8>();
/// fn_under_verification(inputA);
/// ```
///
/// Note: This is a safe construct and can only be used with types that implement the `Arbitrary`
/// trait. The Arbitrary trait is used to build a symbolic value that represents all possible
/// valid values for type `T`.
#[inline(always)]
pub fn any<T: Arbitrary>() -> T {
    T::any()
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
///
/// # Safety
///
/// This function is unsafe and it may represent invalid `T` values which can lead to many
/// undesirable undefined behaviors. Users must validate that the symbolic variable respects
/// the type invariant as well as any other constraint relevant to their usage. E.g.:
///
/// ```rust
/// let c = unsafe { rmc::any_raw::char() };
/// rmc::assume(char::from_u32(c as u32).is_ok());
/// ```
///
#[rustc_diagnostic_item = "RmcAnyRaw"]
#[inline(never)]
pub unsafe fn any_raw<T>() -> T {
    unimplemented!("RMC any_raw")
}

/// This function has been split into a safe and unsafe functions: `rmc::any` and `rmc::any_raw`.
#[deprecated]
#[inline(never)]
pub fn nondet<T: Arbitrary>() -> T {
    any::<T>()
}

/// Function used in tests for cases where the condition is not always true.
#[inline(never)]
#[rustc_diagnostic_item = "RmcExpectFail"]
pub fn expect_fail(_cond: bool, _message: &str) {}

/// RMC proc macros must be in a separate crate
pub use rmc_macros::*;
