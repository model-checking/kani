// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates implementations of the `Arbitrary` trait for various types. The `Arbitrary` trait defines
//! methods for generating arbitrary (unconstrained) values of the implementing type.
//! trivial_arbitrary and nonzero_arbitrary are implementations of Arbitrary for types that can be represented
//! by an unconstrained symbolic value of their size (e.g., `u8`, `u16`, `u32`, etc.).
//!
//! TODO: Use this inside kani library so that we dont have to maintain two copies of the same proc macro for arbitrary.
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

        /// Enumeration with the cases currently covered by the pointer generator.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, crate::kani::Arbitrary)]
        pub enum AllocationStatus {
            /// Dangling pointers
            Dangling,
            /// Pointer to dead object
            DeadObj,
            /// Null pointers
            Null,
            /// In bounds pointer (it may be unaligned)
            InBounds,
            /// Out of bounds
            OutBounds,
        }

        /// Pointer generator that can be used to generate an arbitrary pointer.
        ///
        /// This generator allows users to build pointers with different safety properties.
        /// It contains an internal buffer that it uses to generate `InBounds` and `OutBounds` pointers.
        /// In those cases, the pointers will have the same provenance as the generator, and the same lifetime.
        ///
        /// For example:
        /// ```ignore
        ///     let generator = PointerGenerator<char, 5>::new();
        ///     let arbitrary = generator.generate_ptr();
        ///     kani::assume(arbitrary.status == AllocationStatus::InBounds);
        ///     // Pointer may be unaligned but it should be in-bounds.
        ///     unsafe { arbitrary.ptr.write_unaligned(kani::any() }
        /// ```
        ///
        /// Use the same generator if you want to handle cases where 2 or more pointers may overlap. E.g.:
        /// ```ignore
        ///     let generator = PointerGenerator<char, 5>::new();
        ///     let arbitrary = generator.generate_ptr();
        ///     kani::assume(arbitrary.status == AllocationStatus::InBounds);
        ///     let ptr = arbitrary.ptr;
        ///     kani::cover!(arbitrary.ptr == generator.generate_ptr());
        ///     kani::cover!(arbitrary.ptr != generator.generate_ptr());
        /// ```
        ///
        /// Note: This code is different than generating a pointer with any address. I.e.:
        /// ```ignore
        ///     // This pointer represents any address.
        ///     let ptr = kani::any::<usize>() as *const u8;
        ///     // Which is different from:
        ///     let generator = PointerGenerator<u8, 5>::new();
        ///     let ptr = generator.generate_ptr().ptr;
        /// ```
        ///
        /// Kani cannot reason about a pointer allocation status (except for asserting its validity).
        /// Thus, this interface allow users to write harnesses that impose constraints to the arbitrary pointer.
        #[derive(Debug)]
        pub struct PointerGenerator<T, const BUF_LEN: usize> {
            // Internal allocation that may be used to generate valid pointers.
            buf: MaybeUninit<[T; BUF_LEN]>,
        }

        /// Holds information about a pointer that is generated non-deterministically.
        #[derive(Debug)]
        pub struct ArbitraryPointer<'a, T> {
            /// The pointer that was generated.
            pub ptr: *mut T,
            /// The expected allocation status.
            pub status: AllocationStatus,
            /// Whether the pointer was generated with an initialized value or not.
            pub is_initialized: bool,
            /// Lifetime for this object.
            phantom: PhantomData<&'a T>,
        }

        impl<T: crate::kani::Arbitrary, const BUF_LEN: usize> PointerGenerator<T, BUF_LEN> {
            const _VALID: () = assert!(BUF_LEN > 0, "PointerGenerator requires non-zero length.");

            /// Create a new PointerGenerator.
            #[crate::kani::unstable_feature(
                feature = "mem-predicates",
                issue = 2690,
                reason = "experimental memory predicates and manipulation feature"
            )]
            pub fn new() -> Self {
                // Use constant to trigger static length validation.
                let _ = Self::_VALID;
                PointerGenerator { buf: MaybeUninit::uninit() }
            }

            /// Creates a raw pointer with non-deterministic properties.
            ///
            /// The pointer returned is either dangling or has the same provenance of the generator.
            #[crate::kani::unstable_feature(
                feature = "mem-predicates",
                issue = 2690,
                reason = "experimental memory predicates and manipulation feature"
            )]
            pub fn any_alloc_status<'a>(&'a mut self) -> ArbitraryPointer<'a, T> {
                // Create an arbitrary pointer, but leave `ptr` as unset for now.
                let mut arbitrary = ArbitraryPointer {
                    ptr: ptr::null_mut::<T>(),
                    is_initialized: false,
                    status: crate::kani::any(),
                    phantom: PhantomData,
                };

                let buf_ptr = addr_of_mut!(self.buf) as *mut T;

                // Offset is used to potentially generate unaligned pointer.
                let offset = crate::kani::any_where(|b: &usize| *b < size_of::<T>());
                arbitrary.ptr = match arbitrary.status {
                    AllocationStatus::Dangling => {
                        crate::ptr::NonNull::<T>::dangling().as_ptr().wrapping_add(offset)
                    }
                    AllocationStatus::DeadObj => {
                        let mut obj: T = crate::kani::any();
                        &mut obj as *mut _
                    }
                    AllocationStatus::Null => crate::ptr::null_mut::<T>(),
                    AllocationStatus::InBounds => {
                        // Note that compilation fails if BUF_LEN is 0.
                        let pos = crate::kani::any_where(|i: &usize| *i < (BUF_LEN - 1));
                        let ptr: *mut T = buf_ptr.wrapping_add(pos).wrapping_byte_add(offset);
                        if crate::kani::any() {
                            arbitrary.is_initialized = true;
                            // This should be in bounds of arbitrary.alloc.
                            unsafe { ptr.write_unaligned(crate::kani::any()) };
                        }
                        ptr
                    }
                    AllocationStatus::OutBounds => {
                        if crate::kani::any() {
                            buf_ptr.wrapping_add(BUF_LEN).wrapping_byte_sub(offset)
                        } else {
                            buf_ptr.wrapping_add(BUF_LEN).wrapping_byte_add(offset)
                        }
                    }
                };

                arbitrary
            }

            /// Creates a in-bounds raw pointer with non-deterministic properties.
            ///
            /// The pointer points to an allocated location with the same provenance of the generator.
            /// The pointer may be unaligned, and the pointee may be uninitialized.
            #[crate::kani::unstable_feature(
                feature = "mem-predicates",
                issue = 2690,
                reason = "experimental memory predicates and manipulation feature"
            )]
            pub fn any_in_bounds<'a>(&'a mut self) -> ArbitraryPointer<'a, T> {
                let buf_ptr = addr_of_mut!(self.buf) as *mut T;
                let pos = crate::kani::any_where(|i: &usize| *i < (BUF_LEN - 1));
                let offset = crate::kani::any_where(|b: &usize| *b < size_of::<T>());
                let ptr: *mut T = buf_ptr.wrapping_add(pos).wrapping_byte_add(offset);
                let is_initialized = crate::kani::any();
                if is_initialized {
                    unsafe { ptr.write_unaligned(crate::kani::any()) };
                }
                ArbitraryPointer {
                    ptr,
                    is_initialized,
                    status: AllocationStatus::InBounds,
                    phantom: PhantomData,
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
