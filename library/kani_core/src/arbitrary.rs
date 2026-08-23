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
    () => {
        use core_path::marker::{PhantomData, PhantomPinned};
        use core_path::mem::MaybeUninit;
        use core_path::ptr::{self, addr_of_mut};

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

        /// Generate a raw pointer in a nondeterministic allocation state, pointing to `storage`
        /// in the valid case. The states are:
        /// - null,
        /// - out of bounds of the allocation (one past the end of `storage`, aligned and
        ///   non-null, but not valid for reads or writes),
        /// - valid, i.e., pointing to `storage`, which the caller keeps alive for as long as the
        ///   returned pointer is in use.
        ///
        /// This model is used by the compiler to generate nondeterministic raw pointer values for
        /// automatic harnesses (`kani autoharness`). Note that the returned pointer is aligned in
        /// all states and has valid provenance, so that memory predicates such as
        /// `kani::mem::can_dereference` can reason about it (they do not support pointers cast
        /// from arbitrary integer addresses).
        /// We do not generate pointers to deallocated objects, since Kani's memory predicates
        /// cannot reason about those either ("Kani does not support reasoning about pointer to
        /// unallocated memory"), which would break harnesses for functions with contracts over
        /// their pointer arguments.
        #[kanitool::fn_marker = "AnyPtrModel"]
        #[inline(never)]
        #[doc(hidden)]
        pub fn any_ptr<T>(storage: &mut T) -> *mut T {
            match crate::kani::any::<u8>() {
                0 => ptr::null_mut(),
                1 => (storage as *mut T).wrapping_add(1),
                _ => storage as *mut T,
            }
        }

        /// Generate a slice of nondeterministic length (at most `N`) referring to a prefix of
        /// `storage`, a nondeterministic array that the caller keeps alive for as long as the
        /// returned slice is in use.
        ///
        /// This model is used by the compiler to generate nondeterministic `&[T]` / `&mut [T]`
        /// arguments for automatic harnesses (`kani autoharness`). Note that any verification
        /// result obtained with a bounded value like this one is valid only up to the bound.
        #[kanitool::fn_marker = "AnySliceRefModel"]
        #[inline(never)]
        #[doc(hidden)]
        pub fn any_slice_ref<T, const N: usize>(storage: &mut [T; N]) -> &mut [T] {
            let len: usize = crate::kani::any();
            crate::kani::assume(len <= N);
            &mut storage[..len]
        }

        /// Generate a string slice referring to the longest valid-UTF-8 prefix of `storage`, a
        /// nondeterministic byte array (at most `N` bytes) that the caller keeps alive for as
        /// long as the returned slice is in use. This is the same approach as `String`'s
        /// `BoundedArbitrary` implementation: computing the valid prefix is a deterministic
        /// function of the nondeterministic bytes, which symbolic execution handles well,
        /// whereas *assuming* `str::from_utf8(..).is_ok()` over nondeterministic bytes and
        /// length is intractable even for a handful of bytes. All string contents up to length
        /// `N` are covered: a string of length `k < N` arises from storage whose byte at index
        /// `k` starts an invalid sequence.
        ///
        /// This model is used by the compiler to generate nondeterministic `&str` arguments for
        /// automatic harnesses (`kani autoharness`). Note that any verification result obtained
        /// with a bounded value like this one is valid only up to the bound.
        #[kanitool::fn_marker = "AnyStrRefModel"]
        #[inline(never)]
        #[doc(hidden)]
        pub fn any_str_ref<const N: usize>(storage: &mut [u8; N]) -> &str {
            let valid_len = match core_path::str::from_utf8(storage) {
                Ok(_) => N,
                // `valid_up_to` is always a character boundary.
                Err(e) => e.valid_up_to(),
            };
            // SAFETY: `storage[..valid_len]` is the longest valid UTF-8 prefix of `storage`.
            unsafe { core_path::str::from_utf8_unchecked(&storage[..valid_len]) }
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

        pub mod slice {
            kani_core::slice_generator!();
        }

        mod range_structures {
            use super::{
                Arbitrary,
                core_path::{
                    mem,
                    ops::{Bound, Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive},
                },
            };

            impl<T> Arbitrary for Bound<T>
            where
                T: Arbitrary,
            {
                fn any() -> Self {
                    match u8::any() {
                        0 => Bound::Included(T::any()),
                        1 => Bound::Excluded(T::any()),
                        _ => Bound::Unbounded,
                    }
                }
            }

            impl<T> Arbitrary for Range<T>
            where
                T: Arbitrary,
            {
                fn any() -> Self {
                    T::any()..T::any()
                }
            }

            impl<T> Arbitrary for RangeFrom<T>
            where
                T: Arbitrary,
            {
                fn any() -> Self {
                    T::any()..
                }
            }

            impl<T> Arbitrary for RangeInclusive<T>
            where
                T: Arbitrary,
            {
                fn any() -> Self {
                    T::any()..=T::any()
                }
            }

            impl<T> Arbitrary for RangeTo<T>
            where
                T: Arbitrary,
            {
                fn any() -> Self {
                    ..T::any()
                }
            }

            impl<T> Arbitrary for RangeToInclusive<T>
            where
                T: Arbitrary,
            {
                fn any() -> Self {
                    ..=T::any()
                }
            }
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
