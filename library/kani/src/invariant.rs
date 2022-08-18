// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the Invariant trait.

use crate::Arbitrary;

/// Types that implement a check to ensure its value is valid and safe to be used. See
/// <https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html> for examples of valid values.
///
/// Implementations of Invariant traits must ensure that the current bit values of the given type
/// is valid and that all its invariants hold.
///
/// # Safety
///
/// This trait is unsafe since &self might represent an invalid value. The `is_valid()` function
/// must return `true` if and only if the invariant of its type is held.
///
/// # Deprecation
///
/// We have decided to deprecate this trait in favor of using `Arbitrary` for now. The main
/// benefit of using Invariant was to avoid calling `kani::any_raw()` followed by `kani::assume()`.
/// However, `kani::any_raw()` today cannot guarantee the rust type validity invariants, which
/// could lead to UB in our analysis.
#[deprecated(
    since = "0.8.0",
    note = "With `kani::Invariant`, Kani cannot guarantee that the type respects the language \
    type invariants which may trigger UB. Use `kani::Arbitrary` instead."
)]
pub unsafe trait Invariant {
    /// Check if `&self` holds a valid value that respect the type invariant.
    /// This function must return `true` if and only if `&self` is valid.
    fn is_valid(&self) -> bool;
}

// We cannot apply #[deprecated] to trait impl so add this to ignore the deprecation warnings.
#[allow(deprecated)]
impl<T> Arbitrary for T
where
    T: Invariant,
    // This generic_const_exprs feature lets Rust know the size of generic T.
    [(); std::mem::size_of::<T>()]:,
{
    default fn any() -> Self {
        assert!(
            !cfg!(feature = "concrete_playback"),
            "Calling `any()` on an `Invariant` type is not supported with the executable trace feature."
        );
        let value = unsafe { crate::any_raw_internal::<T, { std::mem::size_of::<T>() }>() };
        crate::assume(value.is_valid());
        value
    }
}
