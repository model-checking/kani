// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `Arbitrary` trait for various types. The `Arbitrary` trait defines
//! methods for generating arbitrary (unconstrained) values of the implementing type.
//! trivial_arbitrary and nonzero_arbitrary are implementations of Arbitrary for types that can be represented
//! by an unconstrained symbolic value of their size (e.g., `u8`, `u16`, `u32`, etc.).
//!
//! TODO: Use this inside kani library so that we dont have to maintain two copies of the same proc macro for arbitrary.
#[macro_export]
macro_rules! generate_arbitrary {
    ($core:path) => {
        use core_path::marker::{PhantomData, PhantomPinned};
        use $core as core_path;

        pub trait Arbitrary
        where
            Self: Sized,
        {
            fn any() -> Self;
            #[cfg(kani_sysroot)]
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
            // the requirement defined in the where clause must appear on the `impl`'s method `any_array`
            // but also on the corresponding trait's method
            where
                [(); core_path::mem::size_of::<[Self; MAX_ARRAY_LENGTH]>()]:,
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
                        unsafe { any_raw_internal::<Self, { core_path::mem::size_of::<Self>() }>() }
                    }
                    // Disable this for standard library since we cannot enable generic constant expr.
                    #[cfg(kani_sysroot)]
                    fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
                    where
                        // `generic_const_exprs` requires all potential errors to be reflected in the signature/header.
                        // We must repeat the expression in the header, to make sure that if the body can fail the header will also fail.
                        [(); { core_path::mem::size_of::<[$type; MAX_ARRAY_LENGTH]>() }]:,
                    {
                        unsafe {
                            any_raw_internal::<
                                [Self; MAX_ARRAY_LENGTH],
                                { core_path::mem::size_of::<[Self; MAX_ARRAY_LENGTH]>() },
                            >()
                        }
                    }
                }
            };
        }

        macro_rules! nonzero_arbitrary {
            ( $type: ty, $base: ty ) => {
                use core_path::num::*;
                impl Arbitrary for $type {
                    #[inline(always)]
                    fn any() -> Self {
                        let val = <$base>::any();
                        assume(val != 0);
                        unsafe { <$type>::new_unchecked(val) }
                    }
                }
            };
        }

        // Generate trivial arbitrary values
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

        // Implement arbitrary for non-trivial types
        impl Arbitrary for bool {
            #[inline(always)]
            fn any() -> Self {
                let byte = u8::any();
                assume(byte < 2);
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
                assume(val <= 0xD7FF || (0xE000..=0x10FFFF).contains(&val));
                unsafe { char::from_u32_unchecked(val) }
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

        impl<T: ?Sized> Arbitrary for PhantomData<T> {
            fn any() -> Self {
                PhantomData
            }
        }

        impl Arbitrary for PhantomPinned {
            fn any() -> Self {
                PhantomPinned
            }
        }

        #[cfg(kani_sysroot)]
        impl<T, const N: usize> Arbitrary for [T; N]
        where
            T: Arbitrary,
            [(); core_path::mem::size_of::<[T; N]>()]:,
        {
            fn any() -> Self {
                T::any_array()
            }
        }
    };
}
