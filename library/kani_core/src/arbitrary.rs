// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `Arbitrary` trait for various types. The `Arbitrary` trait defines
//! methods for generating arbitrary (unconstrained) values of the implementing type.
//! trivial_arbitrary and nonzero_arbitrary are implementations of Arbitrary for types that can be represented
//! by an unconstrained symbolic value of their size (e.g., `u8`, `u16`, `u32`, etc.).
//!
//! TODO: Use this inside kani library so that we dont have to maintain two copies of the same proc macro for arbitrary.

mod pointer;
mod slice;

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_arbitrary {
    ($core:path) => {
        use core_path::marker::{PhantomData, PhantomPinned};
        use core_path::mem::MaybeUninit;
        use core_path::ptr::{self, addr_of_mut};
        use $core as core_path;

        pub trait Arbitrary
        where
            Self: Sized,
        {
            fn any() -> Self;
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH] {
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
                        unsafe { crate::kani::any_raw_internal::<Self>() }
                    }
                    fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH] {
                        unsafe { crate::kani::any_raw_array::<Self, MAX_ARRAY_LENGTH>() }
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
        trivial_arbitrary!(());

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

        // We do not constrain floating points values per type spec. Users must add assumptions to their
        // verification code if they want to eliminate NaN, infinite, or subnormal.
        trivial_arbitrary!(f32);
        trivial_arbitrary!(f64);

        // Similarly, we do not constraint values for non-standard floating types.
        trivial_arbitrary!(f16);
        trivial_arbitrary!(f128);

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

        impl<T, const N: usize> Arbitrary for [T; N]
        where
            T: Arbitrary,
        {
            fn any() -> Self {
                T::any_array::<N>()
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

        impl<T> Arbitrary for MaybeUninit<T>
        where
            T: Arbitrary,
        {
            fn any() -> Self {
                if crate::kani::any() { MaybeUninit::new(T::any()) } else { MaybeUninit::uninit() }
            }
        }

        arbitrary_tuple!(A);
        arbitrary_tuple!(A, B);
        arbitrary_tuple!(A, B, C);
        arbitrary_tuple!(A, B, C, D);
        arbitrary_tuple!(A, B, C, D, E);
        arbitrary_tuple!(A, B, C, D, E, F);
        arbitrary_tuple!(A, B, C, D, E, F, G);
        arbitrary_tuple!(A, B, C, D, E, F, G, H);
        arbitrary_tuple!(A, B, C, D, E, F, G, H, I);
        arbitrary_tuple!(A, B, C, D, E, F, G, H, I, J);
        arbitrary_tuple!(A, B, C, D, E, F, G, H, I, J, K);
        arbitrary_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

        pub use self::arbitrary_ptr::*;
        mod arbitrary_ptr {
            kani_core::ptr_generator!();
        }

        pub use self::arbitrary_slice::*;
        mod arbitrary_slice {
            kani_core::slice_generator!();
        }
    };
}

/// This macro implements `kani::Arbitrary` on a tuple whose elements
/// already implement `kani::Arbitrary` by running `kani::any()` on
/// each index of the tuple.
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! arbitrary_tuple {
    ($($type:ident),*) => {
        impl<$($type : Arbitrary),*>  Arbitrary for ($($type,)*) {
            #[inline(always)]
            fn any() -> Self {
                ($(crate::kani::any::<$type>(),)*)
            }
        }
    }
}
