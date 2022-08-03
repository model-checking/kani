// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(rustc_attrs)] // Used for rustc_diagnostic_item.
#![feature(min_specialization)] // Used for default implementation of Arbitrary.
#![allow(incomplete_features)]
#![feature(generic_const_exprs)] // Used for getting size_of generic types

pub mod arbitrary;
pub mod futures;
pub mod invariant;
pub mod slice;
pub mod vec;

pub use arbitrary::Arbitrary;
pub use futures::block_on;
#[allow(deprecated)]
pub use invariant::Invariant;

#[cfg(feature = "exe_trace")]
use std::cell::RefCell;

#[cfg(feature = "exe_trace")]
// Don't need locking mechanism if using thread local
thread_local! {
    pub static DET_VALS: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
}

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
pub fn assume(_cond: bool) {
    #[cfg(feature = "exe_trace")]
    assert!(_cond);
}

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
pub fn assert(_cond: bool, _msg: &'static str) {
    #[cfg(feature = "exe_trace")]
    assert!(_cond, "{}", _msg);
}

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
/// # Safety
///
/// This function is unsafe and it may represent invalid `T` values which can lead to many
/// undesirable undefined behaviors. Users must guarantee that an unconstrained symbolic value
/// for type T only represents valid values.
///
/// # Deprecation
///
/// We have decided to deprecate this function due to the fact that its result can be the source
/// of undefined behavior.
#[inline(never)]
#[deprecated(
    since = "0.8.0",
    note = "This function may return symbolic values that don't respects the language type invariants."
)]
#[doc(hidden)]
pub unsafe fn any_raw<T>() -> T {
where
    [(); std::mem::size_of::<T>()]:,
{
    any_raw_internal::<T, { std::mem::size_of::<T>() }>()
}

/// This function will replace `any_raw` that has been deprecated and it should only be used
/// internally when we can guarantee that it will not trigger any undefined behavior.
#[rustc_diagnostic_item = "KaniAnyRaw"]
#[inline(never)]
pub(crate) unsafe fn any_raw_internal<T, const SIZE_T>() -> T {
    #[cfg(feature = "exe_trace")]
    {
        let mut bytes_t = [0; SIZE_T];
        DET_VALS.with(|det_vals| {
            let tmp_det_vals = &mut *det_vals.borrow_mut();
            let next_num = tmp_det_vals.pop().expect("Not enough det vals found");
            assert_eq!(next_num.len(), SIZE_T, "Mismatch in num bytes in det val");
            for i in 0..SIZE_T {
                bytes_t[i] = next_num[i];
            }
        });
        return std::mem::transmute_copy::<[u8; SIZE_T], T>(&bytes_t);
    }

    #[cfg(not(feature = "exe_trace"))]
    unimplemented!("Kani any_raw_inner");
}

#[cfg(feature = "exe_trace")]
pub fn exe_trace_init(mut det_vals: Vec<Vec<u8>>) {
    DET_VALS.with(|glob_det_vals| {
        det_vals.reverse();
        let tmp_glob_det_vals = &mut *glob_det_vals.borrow_mut();
        *tmp_glob_det_vals = det_vals;
    });
}

/// Function used in tests for cases where the condition is not always true.
#[inline(never)]
#[rustc_diagnostic_item = "KaniExpectFail"]
pub fn expect_fail(_cond: bool, _message: &'static str) {
    #[cfg(feature = "exe_trace")]
    assert!(!_cond, "{}", _message);
}

/// Function used to generate panic with a static message as this is the only one currently
/// supported by Kani display.
///
/// During verification this will get replaced by `assert(false)`. For concrete executions, we just
/// invoke the regular `std::panic!()` function. This function is used by our standard library
/// overrides, but not the other way around.
#[inline(never)]
#[rustc_diagnostic_item = "KaniPanic"]
#[doc(hidden)]
pub fn panic(message: &'static str) -> ! {
    panic!("{}", message)
}

/// Kani proc macros must be in a separate crate
pub use kani_macros::*;
