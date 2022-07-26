// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the Invariant trait as well as implementation for commonly used types.
use std::num::*;

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
pub unsafe trait Invariant {
    /// Check if `&self` holds a valid value that respect the type invariant.
    /// This function must return `true` if and only if `&self` is valid.
    fn is_valid(&self) -> bool;
}

macro_rules! empty_invariant {
    ( $type: ty ) => {
        unsafe impl Invariant for $type {
            #[inline(always)]
            fn is_valid(&self) -> bool {
                true
            }
        }
    };
}

empty_invariant!(u8);
empty_invariant!(u16);
empty_invariant!(u32);
empty_invariant!(u64);
empty_invariant!(u128);
empty_invariant!(usize);

empty_invariant!(i8);
empty_invariant!(i16);
empty_invariant!(i32);
empty_invariant!(i64);
empty_invariant!(i128);
empty_invariant!(isize);

// We do not constraint floating points values per type spec. Users must add assumptions to their
// verification code if they want to eliminate NaN, infinite, or subnormal.
empty_invariant!(f32);
empty_invariant!(f64);

empty_invariant!(());

unsafe impl Invariant for bool {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        let byte = u8::from(*self);
        byte == 0 || byte == 1
    }
}

/// Validate that a char is not outside the ranges [0x0, 0xD7FF] and [0xE000, 0x10FFFF]
/// Ref: <https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html>
unsafe impl Invariant for char {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        // Kani translates char into i32.
        let val = *self as i32;
        val <= 0xD7FF || (0xE000..=0x10FFFF).contains(&val)
    }
}

unsafe impl<T: Invariant, const N: usize> Invariant for [T; N] {
    fn is_valid(&self) -> bool {
        self.iter().all(|e| e.is_valid())
    }
}

unsafe impl<T> Invariant for Option<T>
where
    T: Invariant,
{
    #[inline(always)]
    fn is_valid(&self) -> bool {
        if let Some(v) = self { v.is_valid() } else { matches!(*self, None) }
    }
}

unsafe impl<T, E> Invariant for Result<T, E>
where
    T: Invariant,
    E: Invariant,
{
    #[inline(always)]
    fn is_valid(&self) -> bool {
        if let Ok(v) = self {
            v.is_valid()
        } else if let Err(e) = self {
            e.is_valid()
        } else {
            false
        }
    }
}

macro_rules! nonzero_invariant {
    ( $type: ty ) => {
        unsafe impl Invariant for $type {
            #[inline(always)]
            fn is_valid(&self) -> bool {
                self.get() != 0
            }
        }
    };
}

nonzero_invariant!(NonZeroU8);
nonzero_invariant!(NonZeroU16);
nonzero_invariant!(NonZeroU32);
nonzero_invariant!(NonZeroU64);
nonzero_invariant!(NonZeroU128);
nonzero_invariant!(NonZeroUsize);

nonzero_invariant!(NonZeroI8);
nonzero_invariant!(NonZeroI16);
nonzero_invariant!(NonZeroI32);
nonzero_invariant!(NonZeroI64);
nonzero_invariant!(NonZeroI128);
nonzero_invariant!(NonZeroIsize);
