// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This macro generates the logic required to generate pointers with arbitrary statuses.
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! ptr_generator {
    () => {
        use core::marker::PhantomData;
        use core::mem::MaybeUninit;
        use core::ptr::{self, addr_of_mut};
        use crate::kani;

        /// Pointer generator that can be used to generate arbitrary pointers.
        ///
        /// This generator allows users to build pointers with different safety properties.
        /// This is different than creating a pointer that can have any address, since it will never
        /// point to a previously allocated object.
        /// See [this section](crate::PointerGenerator#pointer-generator-vs-pointer-with-any-address)
        /// for more details.
        ///
        /// The generator contains an internal buffer of a constant generic size, `BYTES`, that it
        /// uses to generate `InBounds` and `OutOfBounds` pointers.
        /// In those cases, the generated pointers will have the same provenance as the generator,
        /// and the same lifetime.
        /// The address of an `InBounds` pointer will represent all possible addresses in the range
        /// of the generator's buffer address.
        ///
        /// For other allocation statuses, the generator will create a pointer that satisfies the
        /// given condition.
        /// The pointer address will **not** represent all possible addresses that satisfies the
        /// given allocation status.
        ///
        /// For example:
        /// ```no_run
        /// # use kani::*;
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let mut generator = PointerGenerator::<10>::new();
        ///     let arbitrary = generator.any_alloc_status::<char>();
        ///     kani::assume(arbitrary.status == AllocationStatus::InBounds);
        ///     // Pointer may be unaligned, but it should be in-bounds, so it is safe to write to
        ///     unsafe { arbitrary.ptr.write_unaligned(kani::any()) }
        /// # }
        /// ```
        ///
        /// The generator is parameterized by the number of bytes of its internal buffer.
        /// See [pointer_generator] function if you would like to create a generator that fits
        /// a minimum number of objects of a given type. Example:
        ///
        /// ```no_run
        /// # use kani::*;
        /// # #[allow(unused)]
        /// # #[kani::proof]
        /// # fn harness() {
        ///     // These generators have the same capacity of 6 bytes.
        ///     let generator1 = PointerGenerator::<6>::new();
        ///     let generator2 = pointer_generator::<i16, 3>();
        /// # }
        /// ```
        ///
        /// ## Buffer size
        ///
        /// The internal buffer is used to generate pointers, and its size determines the maximum
        /// number of pointers it can generate without overlapping.
        /// Larger values will impact the maximum distance between generated pointers.
        ///
        /// We recommend that you pick a size that is at least big enough to
        /// cover the cases where all pointers produced are non-overlapping.
        /// The buffer size in bytes must be big enough to fit distinct objects for each call
        /// of generate pointer.
        /// For example, generating two `*mut u8` and one `*mut u32` requires a buffer
        /// of at least 6 bytes.
        ///
        /// This guarantees that your harness covers cases where all generated pointers
        /// point to allocated positions that do not overlap. For example:
        ///
        /// ```no_run
        /// # use kani::*;
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let mut generator = PointerGenerator::<6>::new();
        ///     let ptr1: *mut u8 = generator.any_in_bounds().ptr;
        ///     let ptr2: *mut u8 = generator.any_in_bounds().ptr;
        ///     let ptr3: *mut u32 = generator.any_in_bounds().ptr;
        ///     // This cover is satisfied.
        ///     cover!((ptr1 as usize) >= (ptr2 as usize) + size_of::<u8>()
        ///            && (ptr2 as usize) >= (ptr3 as usize) + size_of::<u32>());
        ///     // As well as having overlapping pointers.
        ///     cover!((ptr1 as usize) == (ptr3 as usize));
        /// # }
        /// ```
        ///
        /// The first cover will be satisfied, since there exists at least one path where
        /// the generator produces inbounds pointers that do not overlap. Such as this scenario:
        ///
        /// ```text
        /// +--------+--------+--------+--------+--------+--------+
        /// | Byte 0 | Byte 1 | Byte 2 | Byte 3 | Byte 4 | Byte 5 |
        /// +--------+--------+--------+--------+--------+--------+
        /// <--------------- ptr3 --------------><--ptr2-><--ptr1->
        /// ```
        ///
        /// I.e., the generator buffer is large enough to fit all 3 objects without overlapping.
        ///
        /// In contrast, if we had used a size of 1 element, all calls to `any_in_bounds()` would
        /// return elements that overlap, and the first cover would no longer be satisfied.
        ///
        /// Note that the generator requires a minimum number of 1 byte, otherwise the
        /// `InBounds` case would never be covered.
        /// Compilation will fail if you try to create a generator of size `0`.
        ///
        /// Additionally, the verification will fail if you try to generate a pointer for a type
        /// with size greater than the buffer size.
        ///
        /// Use larger buffer size if you want to cover scenarios where the distance
        /// between the generated pointers matters.
        ///
        /// The only caveats of using very large numbers are:
        ///  1. The value cannot exceed the solver maximum object size (currently 2^48 by default), neither Rust's
        ///     maximum object size (`isize::MAX`).
        ///  2. Larger sizes could impact performance as they can lead to an exponential increase in the number of possibilities of pointer placement within the buffer.
        ///
        /// # Pointer provenance
        ///
        /// The pointer returned in the `InBounds` and `OutOfBounds` case will have the same
        /// provenance as the generator.
        ///
        /// Use the same generator if you want to handle cases where 2 or more pointers may overlap. E.g.:
        /// ```no_run
        /// # use kani::*;
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let mut generator = pointer_generator::<char, 5>();
        ///     let ptr1 = generator.any_in_bounds::<char>().ptr;
        ///     let ptr2 = generator.any_in_bounds::<char>().ptr;
        ///     // This cover is satisfied.
        ///     cover!(ptr1 == ptr2)
        /// # }
        /// ```
        ///
        /// If you want to cover cases where two or more pointers may not have the same
        /// provenance, you will need to instantiate multiple generators.
        /// You can also apply non-determinism to cover cases where the pointers may or may not
        /// have the same provenance. E.g.:
        ///
        /// ```no_run
        /// # use kani::*;
        /// # unsafe fn my_target<T>(_ptr1: *const T, _ptr2: *const T) {}
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let mut generator1 = pointer_generator::<char, 5>();
        ///     let mut generator2 = pointer_generator::<char, 5>();
        ///     let ptr1: *const char = generator1.any_in_bounds().ptr;
        ///     let ptr2: *const char = if kani::any() {
        ///         // Pointers will have same provenance and may overlap.
        ///         generator1.any_in_bounds().ptr
        ///     } else {
        ///         // Pointers will have different provenance and will not overlap.
        ///         generator2.any_in_bounds().ptr
        ///     };
        ///     // Invoke the function under verification
        ///     unsafe { my_target(ptr1, ptr2) };
        /// # }
        /// ```
        ///
        /// # Pointer Generator vs Pointer with any address
        ///
        /// Creating a pointer using the generator is different than generating a pointer
        /// with any address.
        ///
        /// I.e.:
        /// ```no_run
        /// # use kani::*;
        /// # #[kani::proof]
        /// # #[allow(unused)]
        /// # fn harness() {
        ///     // This pointer represents any address, and it may point to anything in memory,
        ///     // allocated or not.
        ///     let ptr1 = kani::any::<usize>() as *const u8;
        ///
        ///     // This pointer address will either point to unallocated memory, to a dead object
        ///     // or to allocated memory within the generator address space.
        ///     let mut generator = PointerGenerator::<5>::new();
        ///     let ptr2: *const u8 = generator.any_alloc_status().ptr;
        /// # }
        /// ```
        ///
        /// Kani cannot reason about a pointer allocation status (except for asserting its validity).
        /// Thus, the generator was introduced to help writing harnesses that need to impose
        /// constraints to the arbitrary pointer allocation status.
        /// It also allow us to restrict the pointer provenance, excluding for example the address of
        /// variables that are not available in the current context.
        /// As a limitation, it will not cover the entire address space that a pointer can take.
        ///
        /// If your harness does not need to reason about pointer allocation, for example, verifying
        /// pointer wrapping arithmetic, using a pointer with any address will allow you to cover
        /// all possible scenarios.
        #[derive(Debug)]
        pub struct PointerGenerator<const BYTES: usize> {
            // Internal allocation that may be used to generate valid pointers.
            buf: MaybeUninit<[u8; BYTES]>,
        }

        /// Enumeration with the cases currently covered by the pointer generator.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, kani::Arbitrary)]
        pub enum AllocationStatus {
            /// Dangling pointers
            Dangling,
            /// Pointer to dead object
            DeadObject,
            /// Null pointers
            Null,
            /// In bounds pointer (it may be unaligned)
            InBounds,
            /// The pointer cannot be read / written to for the given type since one or more bytes
            /// would be out of bounds of the current allocation.
            OutOfBounds,
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

        impl<const BYTES: usize> PointerGenerator<BYTES> {
            const BUF_LEN: usize = BYTES;
            const VALID : () = assert!(BYTES > 0, "PointerGenerator requires at least one byte.");

            /// Create a new PointerGenerator.
            #[kani::unstable_feature(
                feature = "mem-predicates",
                issue = 2690,
                reason = "experimental memory predicates and manipulation feature"
            )]
            pub fn new() -> Self {
                let _ = Self::VALID;
                PointerGenerator { buf: MaybeUninit::uninit() }
            }

            /// Creates a raw pointer with non-deterministic properties.
            ///
            /// The pointer returned is either dangling or has the same provenance of the generator.
            #[kani::unstable_feature(
                feature = "mem-predicates",
                issue = 2690,
                reason = "experimental memory predicates and manipulation feature"
            )]
            pub fn any_alloc_status<'a, T>(&'a mut self) -> ArbitraryPointer<'a, T>
             where T: kani::Arbitrary
            {
                assert!(core::mem::size_of::<T>() <= Self::BUF_LEN,
                    "Cannot generate in-bounds object of the requested type. Buffer is not big enough."
                );

                let status = kani::any();
                let ptr = match status {
                    AllocationStatus::Dangling => {
                        // Generate potentially unaligned pointer.
                        let offset = kani::any_where(|b: &usize| *b < size_of::<T>());
                        crate::ptr::NonNull::<T>::dangling().as_ptr().wrapping_add(offset)
                    }
                    AllocationStatus::DeadObject => {
                        let mut obj: T = kani::any();
                        &mut obj as *mut _
                    }
                    AllocationStatus::Null => crate::ptr::null_mut::<T>(),
                    AllocationStatus::InBounds => {
                        return self.create_in_bounds_ptr();
                    }
                    AllocationStatus::OutOfBounds => {
                        // Generate potentially unaligned pointer.
                        let buf_ptr = addr_of_mut!(self.buf) as *mut u8;
                        let offset = kani::any_where(|b: &usize| *b < size_of::<T>());
                        unsafe { buf_ptr.add(Self::BUF_LEN - offset) as *mut T }
                    }
                };

                ArbitraryPointer {
                    ptr,
                    is_initialized: false,
                    status,
                    phantom: PhantomData,
                }
            }

            /// Creates a in-bounds raw pointer with non-deterministic properties.
            ///
            /// The pointer points to an allocated location with the same provenance of the generator.
            /// The pointer may be unaligned, and the pointee may be uninitialized.
            ///
            /// ```no_run
            /// # use kani::*;
            /// # #[kani::proof]
            /// # fn check_distance() {
            ///     let mut generator = PointerGenerator::<6>::new();
            ///     let ptr1: *mut u8 = generator.any_in_bounds().ptr;
            ///     let ptr2: *mut u8 = generator.any_in_bounds().ptr;
            ///     // SAFETY: Both pointers have the same provenance.
            ///     let distance = unsafe { ptr1.offset_from(ptr2) };
            ///     assert!(distance > -5 && distance < 5)
            /// # }
            /// ```
            #[kani::unstable_feature(
                feature = "mem-predicates",
                issue = 2690,
                reason = "experimental memory predicates and manipulation feature"
            )]
            pub fn any_in_bounds<'a, T>(&'a mut self) -> ArbitraryPointer<'a, T>
            where T: kani::Arbitrary {
                assert!(core::mem::size_of::<T>() <= Self::BUF_LEN,
                    "Cannot generate in-bounds object of the requested type. Buffer is not big enough."
                );
                self.create_in_bounds_ptr()
            }

            /// This is the inner logic to create an arbitrary pointer that is inbounds.
            ///
            /// Note that pointer may be unaligned.
            fn create_in_bounds_ptr<'a, T>(&'a mut self) -> ArbitraryPointer<'a, T>
            where T: kani::Arbitrary {
                assert!(core::mem::size_of::<T>() <= Self::BUF_LEN,
                    "Cannot generate in-bounds object of the requested type. Buffer is not big enough."
                );
                let buf_ptr = addr_of_mut!(self.buf) as *mut u8;
                let offset = kani::any_where(|b: &usize| *b <= Self::BUF_LEN - size_of::<T>());
                let ptr = unsafe { buf_ptr.add(offset) as *mut T };
                let is_initialized = kani::any();
                if is_initialized {
                    unsafe { ptr.write_unaligned(kani::any()) };
                }
                ArbitraryPointer {
                    ptr,
                    is_initialized,
                    status: AllocationStatus::InBounds,
                    phantom: PhantomData,
                }
            }
        }

        kani_core::ptr_generator_fn!();
    };
}

#[cfg(not(feature = "no_core"))]
#[macro_export]
macro_rules! ptr_generator_fn {
    () => {
        /// Create a pointer generator that fits at least `N` elements of type `T`.
        pub fn pointer_generator<T, const NUM_ELTS: usize>()
        -> PointerGenerator<{ size_of::<T>() * NUM_ELTS }> {
            PointerGenerator::<{ size_of::<T>() * NUM_ELTS }>::new()
        }
    };
}

/// Don't generate the pointer_generator function here since it requires generic constant
/// expression.
#[cfg(feature = "no_core")]
#[macro_export]
macro_rules! ptr_generator_fn {
    () => {};
}
