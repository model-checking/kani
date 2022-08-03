// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the Arbitrary trait as well as implementation for primitive types and
//! other std containers.
use std::num::*;

/// This trait should be used to generate symbolic variables that represent any valid value of
/// its type.
pub trait Arbitrary {
    fn any() -> Self;
}

/// The given type can be represented by an unconstrained symbolic value of size_of::<T>.
macro_rules! trivial_arbitrary {
    ( $type: ty ) => {
        impl Arbitrary for $type {
            #[inline(always)]
            fn any() -> Self {
                unsafe { crate::any_raw_internal::<$type>() }
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
        byte == 0
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
{
    fn any() -> Self {
        [(); N].map(|_| T::any())
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

macro_rules! tuple_arbitrary {
    ($($typ:ident),*) => {
        impl<$($typ : Arbitrary),*> Arbitrary for ($($typ,)*) {
            fn any() -> Self {
                ($($typ::any(),)*)
            }
        }
    }
}

tuple_arbitrary!(A);
tuple_arbitrary!(A, B);
tuple_arbitrary!(A, B, C);
tuple_arbitrary!(A, B, C, D);
tuple_arbitrary!(A, B, C, D, E);
tuple_arbitrary!(A, B, C, D, E, F);
tuple_arbitrary!(A, B, C, D, E, F, G);
tuple_arbitrary!(A, B, C, D, E, F, G, H);
tuple_arbitrary!(A, B, C, D, E, F, G, H, I);
tuple_arbitrary!(A, B, C, D, E, F, G, H, I, J);
tuple_arbitrary!(A, B, C, D, E, F, G, H, I, J, K);
tuple_arbitrary!(A, B, C, D, E, F, G, H, I, J, K, L);
