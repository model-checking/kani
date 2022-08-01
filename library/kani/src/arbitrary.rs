// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the Arbitrary trait as well as implementation for the Invariant trait.
use crate::{any_raw, assume, Invariant};

/// This trait should be used to generate symbolic variables that represent any valid value of
/// its type.
pub trait Arbitrary {
    fn any() -> Self;
}

impl<T> Arbitrary for T
where
    T: Invariant,
    [(); std::mem::size_of::<T>()]:,
{
    default fn any() -> Self {
        let value = unsafe { any_raw::<T>() };
        assume(value.is_valid());
        value
    }
}

impl<T, const N: usize> Arbitrary for [T; N]
where
    T: Arbitrary,
    [(); std::mem::size_of::<[T; N]>()]:,
{
    fn any() -> Self {
        // The "correct way" would be to MaybeUninit but there is performance penalty.
        let mut data: [T; N] = unsafe { crate::any_raw() };

        for elem in &mut data[..] {
            *elem = T::any();
        }

        data
    }
}

impl<T> Arbitrary for Option<T>
where
    T: Arbitrary,
{
    fn any() -> Self {
        if bool::any() { Some(T::any()) } else { None }
    }
}

impl<T, E> Arbitrary for Result<T, E>
where
    T: Arbitrary,
    E: Arbitrary,
{
    fn any() -> Self {
        if bool::any() { Ok(T::any()) } else { Err(E::any()) }
    }
}
