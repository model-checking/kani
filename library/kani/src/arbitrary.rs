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
{
    default fn any() -> Self {
        let value = unsafe { any_raw::<T>() };
        assume(value.is_valid());
        value
    }
}
