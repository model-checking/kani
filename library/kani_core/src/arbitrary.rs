// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// The given type can be represented by an unconstrained symbolic value of size_of::<T>.
#[macro_export]
macro_rules! trivial_arbitrary {
    ( $type: ty ) => {
        impl Arbitrary for $type {
            #[inline(always)]
            fn any() -> Self {
                // This size_of call does not use generic_const_exprs feature. It's inside a macro, and Self isn't generic.
                unsafe { any_raw_internal::<Self, { mem_mod::size_of::<Self>() }>() }
            }
            // Disable this for standard library since we cannot enable generic constant expr.
            #[cfg(kani_lib)]
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
            where
                // `generic_const_exprs` requires all potential errors to be reflected in the signature/header.
                // We must repeat the expression in the header, to make sure that if the body can fail the header will also fail.
                [(); { mem_mod::size_of::<[$type; MAX_ARRAY_LENGTH]>() }]:,
            {
                unsafe {
                    any_raw_internal::<
                        [Self; MAX_ARRAY_LENGTH],
                        { mem_mod::size_of::<[Self; MAX_ARRAY_LENGTH]>() },
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
                assume(val != 0);
                unsafe { <$type>::new_unchecked(val) }
            }
        }
    };
}


#[macro_export]
macro_rules! generate_arbitrary {
    ($mem_path:path) => {
        use $mem_path as mem_mod;

        pub trait Arbitrary
        where
            Self: Sized,
        {
            fn any() -> Self;
            #[cfg(kani_lib)]
            fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH]
            // the requirement defined in the where clause must appear on the `impl`'s method `any_array`
            // but also on the corresponding trait's method
            where
                [(); mem_mod::size_of::<[Self; MAX_ARRAY_LENGTH]>()]:,
            {
                [(); MAX_ARRAY_LENGTH].map(|_| Self::any())
            }
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

        #[cfg(kani_lib)]
        impl<T, const N: usize> Arbitrary for [T; N]
        where
            T: Arbitrary,
            [(); mem_mod::size_of::<[T; N]>()]:,
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
