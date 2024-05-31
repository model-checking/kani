// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// The given type can be represented by an unconstrained symbolic value of size_of::<T>.
#[macro_export]
macro_rules! trivial_arbitrary {
    ( $type: ty, $mem_path:path ) => {
        impl Arbitrary for $type {
            #[inline(always)]
            fn any() -> Self {
                // This size_of call does not use generic_const_exprs feature. It's inside a macro, and Self isn't generic.
                unsafe { kani_intrinsics!().any_raw_internal::<Self, { $mem_path::size_of::<Self>() }>() }
            }
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
            where
                // `generic_const_exprs` requires all potential errors to be reflected in the signature/header.
                // We must repeat the expression in the header, to make sure that if the body can fail the header will also fail.
                [(); { $mem_path::size_of::<[$type; MAX_ARRAY_LENGTH]>() }]:,
            {
                unsafe {
                    kani_intrinsics!().any_raw_internal::<
                        [Self; MAX_ARRAY_LENGTH],
                        { $mem_path::size_of::<[Self; MAX_ARRAY_LENGTH]>() },
                    >()
                }
            }
        }
    };
}

#[macro_export]
macro_rules! nonzero_arbitrary {
    ( $type: ty, $base: ty ) => {
        impl Arbitrary for $type {
            #[inline(always)]
            fn any() -> Self {
                let val = <$base>::any();
                kani_intrinsics!().assume(val != 0);
                unsafe { <$type>::new_unchecked(val) }
            }
        }
    };
}


#[macro_export]
macro_rules! generate_arbitrary {
    ($mem_path:path) => {
        pub trait Arbitrary
        where
            Self: Sized,
        {
            fn any() -> Self;
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
            // the requirement defined in the where clause must appear on the `impl`'s method `any_array`
            // but also on the corresponding trait's method
            where
                [(); $mem_path::size_of::<[Self; MAX_ARRAY_LENGTH]>()]:,
            {
                [(); MAX_ARRAY_LENGTH].map(|_| Self::any())
            }
        }

        // Generate trivial arbitrary values
        trivial_arbitrary!(u8, $mem_path);
        trivial_arbitrary!(u16, $mem_path);
        trivial_arbitrary!(u32, $mem_path);
        trivial_arbitrary!(u64, $mem_path);
        trivial_arbitrary!(u128, $mem_path);
        trivial_arbitrary!(usize, $mem_path);

        trivial_arbitrary!(i8, $mem_path);
        trivial_arbitrary!(i16, $mem_path);
        trivial_arbitrary!(i32, $mem_path);
        trivial_arbitrary!(i64, $mem_path);
        trivial_arbitrary!(i128, $mem_path);
        trivial_arbitrary!(isize, $mem_path);

        // Generate nonzero arbitrary implementations
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
                kani_intrinsics!().assume(byte < 2);
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
                kani_intrinsics!().assume(val <= 0xD7FF || (0xE000..=0x10FFFF).contains(&val));
                unsafe { char::from_u32_unchecked(val) }
            }
        }

        impl<T, const N: usize> Arbitrary for [T; N]
        where
            T: Arbitrary,
            [(); $mem_path::size_of::<[T; N]>()]:,
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
    };
}
