// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Required so we can use kani_macros attributes.
#![feature(register_tool)]
#![register_tool(kanitool)]
// Used for rustc_diagnostic_item.
#![feature(rustc_attrs)]
// This is required for the optimized version of `any_array()`
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

pub mod arbitrary;
#[cfg(feature = "concrete_playback")]
mod concrete_playback;
pub mod futures;
pub mod slice;
pub mod tuple;
pub mod vec;

pub use arbitrary::Arbitrary;
#[cfg(feature = "concrete_playback")]
pub use concrete_playback::concrete_playback_run;
#[cfg(not(feature = "concrete_playback"))]
/// NOP `concrete_playback` for type checking during verification mode.
pub fn concrete_playback_run<F: Fn()>(_: Vec<Vec<u8>>, _: F) {
    unreachable!("Concrete playback does not work during verification")
}

pub use futures::block_on;

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
#[cfg(not(feature = "concrete_playback"))]
pub fn assume(cond: bool) {
    let _ = cond;
}

#[inline(never)]
#[rustc_diagnostic_item = "KaniAssume"]
#[cfg(feature = "concrete_playback")]
pub fn assume(cond: bool) {
    assert!(cond, "`kani::assume` should always hold");
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
#[cfg(not(feature = "concrete_playback"))]
#[inline(never)]
#[rustc_diagnostic_item = "KaniAssert"]
pub const fn assert(cond: bool, msg: &'static str) {
    let _ = cond;
    let _ = msg;
}

#[cfg(feature = "concrete_playback")]
#[inline(never)]
#[rustc_diagnostic_item = "KaniAssert"]
pub const fn assert(cond: bool, msg: &'static str) {
    assert!(cond, "{}", msg);
}

/// Creates a cover property with the specified condition and message.
///
/// # Example:
///
/// ```rust
/// kani::cover(slice.len() == 0, "The slice may have a length of 0");
/// ```
///
/// A cover property checks if there is at least one execution that satisfies
/// the specified condition at the location in which the function is called.
///
/// Cover properties are reported as:
///  - SATISFIED: if Kani found an execution that satisfies the condition
///  - UNSATISFIABLE: if Kani proved that the condition cannot be satisfied
///  - UNREACHABLE: if Kani proved that the cover property itself is unreachable (i.e. it is vacuously UNSATISFIABLE)
///
/// This function is called by the [`cover!`] macro. The macro is more
/// convenient to use.
///
#[inline(never)]
#[rustc_diagnostic_item = "KaniCover"]
pub fn cover(_cond: bool, _msg: &'static str) {}

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

/// This creates a symbolic *valid* value of type `T`.
/// The value is constrained to be a value accepted by the predicate passed to the filter.
/// You can assign the return value of this function to a variable that you want to make symbolic.
///
/// # Example:
///
/// In the snippet below, we are verifying the behavior of the function `fn_under_verification`
/// under all possible `u8` input values between 0 and 12.
///
/// ```rust
/// let inputA: u8 = kani::any_where(|x| *x < 12);
/// fn_under_verification(inputA);
/// ```
///
/// Note: This is a safe construct and can only be used with types that implement the `Arbitrary`
/// trait. The Arbitrary trait is used to build a symbolic value that represents all possible
/// valid values for type `T`.
#[inline(always)]
pub fn any_where<T: Arbitrary, F: FnOnce(&T) -> bool>(f: F) -> T {
    let result = T::any();
    assume(f(&result));
    result
}

/// This function creates a symbolic value of type `T`. This may result in an invalid value.
///
/// # Safety
///
/// This function is unsafe and it may represent invalid `T` values which can lead to many
/// undesirable undefined behaviors. Because of that, this function can only be used
/// internally when we can guarantee that the type T has no restriction regarding its bit level
/// representation.
///
/// This function is also used to find concrete values in the CBMC output trace
/// and return those concrete values in concrete playback mode.
///
/// Note that SIZE_T must be equal the size of type T in bytes.
#[inline(never)]
#[cfg(not(feature = "concrete_playback"))]
pub(crate) unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
    any_raw_inner::<T>()
}

#[inline(never)]
#[cfg(feature = "concrete_playback")]
pub(crate) unsafe fn any_raw_internal<T, const SIZE_T: usize>() -> T {
    concrete_playback::any_raw_internal::<T, SIZE_T>()
}

/// This low-level function returns nondet bytes of size T.
#[rustc_diagnostic_item = "KaniAnyRaw"]
#[inline(never)]
#[allow(dead_code)]
fn any_raw_inner<T>() -> T {
    // while we could use `unreachable!()` or `panic!()` as the body of this
    // function, both cause Kani to produce a warning on any program that uses
    // kani::any() (see https://github.com/model-checking/kani/issues/2010).
    // This function is handled via a hook anyway, so we just need to put a body
    // that rustc does not complain about. An infinite loop works out nicely.
    #[allow(clippy::empty_loop)]
    loop {}
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
pub const fn panic(message: &'static str) -> ! {
    panic!("{}", message)
}

/// A macro to check if a condition is satisfiable at a specific location in the
/// code.
///
/// # Example 1:
///
/// ```rust
/// let mut set: BTreeSet<i32> = BTreeSet::new();
/// set.insert(kani::any());
/// set.insert(kani::any());
/// // check if the set can end up with a single element (if both elements
/// // inserted were the same)
/// kani::cover!(set.len() == 1);
/// ```
/// The macro can also be called without any arguments to check if a location is
/// reachable.
///
/// # Example 2:
///
/// ```rust
/// match e {
///     MyEnum::A => { /* .. */ }
///     MyEnum::B => {
///         // make sure the `MyEnum::B` variant is possible
///         kani::cover!();
///         // ..
///     }
/// }
/// ```
///
/// A custom message can also be passed to the macro.
///
/// # Example 3:
///
/// ```rust
/// kani::cover!(x > y, "x can be greater than y")
/// ```
#[macro_export]
macro_rules! cover {
    () => {
        kani::cover(true, "cover location");
    };
    ($cond:expr $(,)?) => {
        kani::cover($cond, concat!("cover condition: ", stringify!($cond)));
    };
    ($cond:expr, $msg:literal) => {
        kani::cover($cond, $msg);
    };
}

/// Kani proc macros must be in a separate crate
pub use kani_macros::*;
