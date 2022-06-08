// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)] // Used for rustc_diagnostic_item.
#![feature(min_specialization)] // Used for default implementation of Arbitrary.
#![feature(generic_const_exprs)] // Used for getting size_of generic types

pub mod arbitrary;
pub mod invariant;
pub mod slice;
pub mod vec;

pub use arbitrary::Arbitrary;
pub use invariant::Invariant;

use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Creates an assumption that will be valid after this statement run. Note that the assumption
/// will only be applied for paths that follow the assumption. If the assumption doesn't hold, the
/// program will exit successfully.
///
/// # Example:
///
/// The code snippet below should never panic.
///
/// ```rust
/// let i : i32 = kani::any();
/// kani::assume(i > 10);
/// if i < 0 {
///   panic!("This will never panic");
/// }
/// ```
///
/// The following code may panic though:
///
/// ```rust
/// let i : i32 = kani::any();
/// assert!(i < 0, "This may panic and verification should fail.");
/// kani::assume(i > 10);
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "KaniAssume"]
pub fn assume(_cond: bool) {}

/// Creates an assertion of the specified condition and message.
///
/// # Example:
///
/// ```rust
/// let x: bool = kani::any();
/// let y = !x;
/// kani::assert(x || y, "ORing a boolean variable with its negation must be true")
/// ```
#[inline(never)]
#[rustc_diagnostic_item = "KaniAssert"]
pub fn assert(_cond: bool, _msg: &'static str) {}

/// This creates an symbolic *valid* value of type `T`. You can assign the return value of this
/// function to a variable that you want to make symbolic.
///
/// # Example:
///
/// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
/// under all possible `NonZeroU8` input values, i.e., all possible `u8` values except zero.
///
/// ```rust
/// let inputA = kani::any::<std::num::NonZeroU8>();
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
/// let inputA = unsafe { kani::any_raw::<char>() };
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
/// let c = unsafe { kani::any_raw::char() };
/// kani::assume(char::from_u32(c as u32).is_ok());
/// ```
///
#[inline(never)]
pub unsafe fn any_raw<T>() -> T
where
    [(); std::mem::size_of::<T>()]:,
{
    let non_det_byte_arr = any_raw_inner::<{ std::mem::size_of::<T>() }>();
    let non_det_var =
        std::mem::transmute_copy::<[u8; std::mem::size_of::<T>()], T>(&non_det_byte_arr);
    non_det_var
}

static RAW_COUNT: AtomicUsize = AtomicUsize::new(0);

#[rustc_diagnostic_item = "KaniAnyRaw"]
#[inline(never)]
unsafe fn any_raw_inner<const T: usize>() -> [u8; T] {
    let det_vals_file = env::var("DET_VALS_FILE").unwrap();
    let det_vals_err_msg = format!("Couldn't read {}", det_vals_file);
    let contents_str = fs::read_to_string(det_vals_file).expect(&det_vals_err_msg);
    let contents_vec: Vec<&str> = contents_str.split("\n").collect();
    let raw_count = RAW_COUNT.fetch_add(T, Ordering::SeqCst);
    let mut bytes_t = [0; T];

    for i in 0..T {
        let a_byte_quotes = contents_vec[raw_count + i];
        let a_byte_quotes_len = a_byte_quotes.len();
        let a_byte_str = &a_byte_quotes[1..a_byte_quotes_len - 1];
        let a_byte: u8 = a_byte_str.parse().unwrap();
        bytes_t[i] = a_byte;
    }
    bytes_t
}

/// This function has been split into a safe and unsafe functions: `kani::any` and `kani::any_raw`.
#[deprecated]
#[inline(never)]
pub fn nondet<T: Arbitrary>() -> T {
    any::<T>()
}

/// Function used in tests for cases where the condition is not always true.
#[inline(never)]
#[rustc_diagnostic_item = "KaniExpectFail"]
pub fn expect_fail(_cond: bool, _message: &'static str) {}

/// Function used to generate panic with a static message as this is the only one currently
/// supported by Kani display.
///
/// During verification this will get replaced by `assert(false)`. For concrete executions, we just
/// invoke the regular `std::panic!()` function. This function is used by our standard library
/// overrides, but not the other way around.
#[inline(never)]
#[rustc_diagnostic_item = "KaniPanic"]
pub fn panic(message: &'static str) -> ! {
    panic!("{}", message)
}

/// Kani proc macros must be in a separate crate
pub use kani_macros::*;
