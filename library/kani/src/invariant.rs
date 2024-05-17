// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the `Invariant` trait as well as its implementation
//! for primitive types.

/// This trait should be used to specify and check type safety invariants for a
/// type.
///
/// Note: Type safety might be checked automatically by Kani in the future.
pub trait Invariant
where
    Self: Sized,
{
    fn is_safe(&self) -> bool;
}

impl Invariant for bool {
    #[inline(always)]
    fn is_safe(&self) -> bool {
        let value = *self as u8;
        value < 2
    }
}

impl Invariant for char {
    #[inline(always)]
    fn is_safe(&self) -> bool {
        let value = *self as u32;
        value <= 0xD7FF || (0xE000..=0x10FFFF).contains(&value)
    }
}

/// Any value is considered safe for the type
macro_rules! trivial_invariant {
    ( $type: ty ) => {
        impl Invariant for $type {
            #[inline(always)]
            fn is_safe(&self) -> bool {
                true
            }
        }
    };
}

trivial_invariant!(u8);
trivial_invariant!(u16);
trivial_invariant!(u32);
trivial_invariant!(u64);
trivial_invariant!(u128);
trivial_invariant!(usize);

trivial_invariant!(i8);
trivial_invariant!(i16);
trivial_invariant!(i32);
trivial_invariant!(i64);
trivial_invariant!(i128);
trivial_invariant!(isize);

// We do not constrain the safety invariant for floating points types.
// Users can create a new type wrapping the floating point type and define an
// invariant that checks for NaN, infinite, or subnormal values.
trivial_invariant!(f32);
trivial_invariant!(f64);

trivial_invariant!(());
