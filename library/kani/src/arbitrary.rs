// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the Arbitrary trait as well as implementation for primitive types and
//! other std containers.

use std::{
    marker::{PhantomData, PhantomPinned},
    num::*,
};

/// This trait should be used to generate symbolic variables that represent any valid value of
/// its type.
pub trait Arbitrary
where
    Self: Sized,
{
    fn any() -> Self;
    fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
    // the requirement defined in the where clause must appear on the `impl`'s method `any_array`
    // but also on the corresponding trait's method
    where
        [(); std::mem::size_of::<[Self; MAX_ARRAY_LENGTH]>()]:,
    {
        [(); MAX_ARRAY_LENGTH].map(|_| Self::any())
    }
}

/// The given type can be represented by an unconstrained symbolic value of size_of::<T>.
macro_rules! trivial_arbitrary {
    ( $type: ty ) => {
        impl Arbitrary for $type {
            #[inline(always)]
            fn any() -> Self {
                // This size_of call does not use generic_const_exprs feature. It's inside a macro, and Self isn't generic.
                unsafe { crate::any_raw_internal::<Self, { std::mem::size_of::<Self>() }>() }
            }
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
            where
                // `generic_const_exprs` requires all potential errors to be reflected in the signature/header.
                // We must repeat the expression in the header, to make sure that if the body can fail the header will also fail.
                [(); { std::mem::size_of::<[$type; MAX_ARRAY_LENGTH]>() }]:,
            {
                unsafe {
                    crate::any_raw_internal::<
                        [Self; MAX_ARRAY_LENGTH],
                        { std::mem::size_of::<[Self; MAX_ARRAY_LENGTH]>() },
                    >()
                }
            }
        }
    };
}

trivial_arbitrary!(u8);
trivial_arbitrary!(u16);
trivial_arbitrary!(u32);
trivial_arbitrary!(u64);
trivial_arbitrary!(u128);
trivial_arbitrary!(usize);

trivial_arbitrary!(i8);
trivial_arbitrary!(i16);
trivial_arbitrary!(i32);
trivial_arbitrary!(i64);
trivial_arbitrary!(i128);
trivial_arbitrary!(isize);

// We do not constraint floating points values per type spec. Users must add assumptions to their
// verification code if they want to eliminate NaN, infinite, or subnormal.
trivial_arbitrary!(f32);
trivial_arbitrary!(f64);

trivial_arbitrary!(());

impl Arbitrary for bool {
    #[inline(always)]
    fn any() -> Self {
        let byte = u8::any();
        crate::assume(byte < 2);
        byte == 1
    }
}

/// Validate that a char is not outside the ranges [0x0, 0xD7FF] and [0xE000, 0x10FFFF]
/// Ref: <https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html>
impl Arbitrary for char {
    #[inline(always)]
    fn any() -> Self {
        // Generate an arbitrary u32 and constrain it to make it a valid representation of char.
        let val = u32::any();
        crate::assume(val <= 0xD7FF || (0xE000..=0x10FFFF).contains(&val));
        unsafe { char::from_u32_unchecked(val) }
    }
}

macro_rules! nonzero_arbitrary {
    ( $type: ty, $base: ty ) => {
        impl Arbitrary for $type {
            #[inline(always)]
            fn any() -> Self {
                let val = <$base>::any();
                crate::assume(val != 0);
                unsafe { <$type>::new_unchecked(val) }
            }
        }
    };
}

nonzero_arbitrary!(NonZeroU8, u8);
nonzero_arbitrary!(NonZeroU16, u16);
nonzero_arbitrary!(NonZeroU32, u32);
nonzero_arbitrary!(NonZeroU64, u64);
nonzero_arbitrary!(NonZeroU128, u128);
nonzero_arbitrary!(NonZeroUsize, usize);

nonzero_arbitrary!(NonZeroI8, i8);
nonzero_arbitrary!(NonZeroI16, i16);
nonzero_arbitrary!(NonZeroI32, i32);
nonzero_arbitrary!(NonZeroI64, i64);
nonzero_arbitrary!(NonZeroI128, i128);
nonzero_arbitrary!(NonZeroIsize, isize);

impl<T, const N: usize> Arbitrary for [T; N]
where
    T: Arbitrary,
    [(); std::mem::size_of::<[T; N]>()]:,
{
    fn any() -> Self {
        T::any_array()
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

impl<T: ?Sized> Arbitrary for std::marker::PhantomData<T> {
    fn any() -> Self {
        PhantomData
    }
}

impl Arbitrary for std::marker::PhantomPinned {
    fn any() -> Self {
        PhantomPinned
    }
}

impl<T> Arbitrary for std::boxed::Box<T>
where
    T: Arbitrary,
{
    fn any() -> Self {
        Box::new(T::any())
    }
}
