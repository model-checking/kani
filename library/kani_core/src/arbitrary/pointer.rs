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

        /// Pointer generator that can be used to generate arbitrary pointers.
        ///
        /// This generator allows users to build pointers with different safety properties.
        /// It contains an internal buffer that it uses to generate `InBounds` and `OutOfBounds` pointers.
        /// In those cases, the pointers will have the same provenance as the generator, and the same lifetime.
        ///
        /// For example:
        /// ```ignore
        /// # use kani::*;
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let generator = PointerGenerator<char, 5>::new();
        ///     let arbitrary = generator.any_alloc_status();
        ///     kani::assume(arbitrary.status == AllocationStatus::InBounds);
        ///     // Pointer may be unaligned but it should be in-bounds.
        ///     unsafe { arbitrary.ptr.write_unaligned(kani::any()) }
        /// # }
        /// ```
        ///
        /// The generator takes a type for the pointers that will be generated, as well as a
        /// number of elements that it can hold without overlapping.
        ///
        /// ## Number of Elements
        ///
        /// The number of elements determine the size of the internal buffer used to generate
        /// pointers. Larger values will cover more cases related to the distance between each
        /// pointer that is generated.
        ///
        /// We recommend this number to be at least greater than the number of pointers that
        /// your harness generate.
        /// This guarantees that your harness covers cases where all generated pointers
        /// points to allocated positions that do not overlap.
        ///
        /// For example, le't say your harness calls `any_in_bounds()` 3 times, and your generator
        /// has 5 elements. Something like:
        ///
        /// ```ignore
        /// # use kani::*;
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let generator = PointerGenerator<char, 5>::new();
        ///     let ptr1 = generator.any_in_bounds().ptr;
        ///     let ptr2 = generator.any_in_bounds().ptr;
        ///     let ptr3 = generator.any_in_bounds().ptr;
        ///     // This cover is satisfied.
        ///     cover!((ptr1 as usize) > (ptr2 as usize) + size_of::<char>()
        ///            && (ptr2 as usize) > (ptr3 as usize) + size_of::<char>());
        /// # }
        /// ```
        ///
        /// The cover statement will be satisfied, since there exists at least one path where
        /// the generator produces inbounds pointers that do not overlap.
        /// I.e., the generator buffer is large enough to fit all 3 objects without overlapping.
        ///
        /// In contrast, if we had used a size of 1 element, all calls to `any_in_bounds()` would
        /// return elements that overlap.
        ///
        /// Note that the generator requires a minimum number of 1 element, otherwise the
        /// `InBounds` case would never be covered.
        /// Compilation will fail if you try to create a generator of size `0`.
        ///
        /// Use larger number of elements if you want to cover scenarios where the distance
        /// between the generated pointers matters.
        ///
        /// The maximum distance between two generated pointers will be
        /// `(NUM_ELTS - 2) * size_of::<T>()` bytes
        ///
        /// # Pointer provenance
        ///
        /// The pointer returned in the `InBounds` and `OutOfBounds` case will have the same
        /// provenance as the generator.
        ///
        /// Use the same generator if you want to handle cases where 2 or more pointers may overlap. E.g.:
        /// ```ignore
        /// # use kani::*;
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let generator = PointerGenerator<char, 5>::new();
        ///     let ptr1 = generator.any_in_bounds().ptr;
        ///     let ptr2 = generator.any_in_bounds().ptr;
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
        /// ```ignore
        /// # use kani::*;
        /// # unsafe fn my_target<T>(_ptr1: *const T; _ptr2: *const T) {}
        /// # #[kani::proof]
        /// # fn harness() {
        ///     let generator1 = PointerGenerator<char, 5>::new();
        ///     let generator2 = PointerGenerator<char, 5>::new();
        ///     let ptr1 = generator1.any_in_bounds().ptr;
        ///     let ptr2 = if kani::any() {
        ///         // Pointers will have same provenance and may overlap.
        ///         generator1.any_in_bounds().ptr;
        ///     } else {
        ///         // Pointers will have different provenance and will not overlap.
        ///         generator2.any_in_bounds().ptr;
        ///     }
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
        /// ```ignore
        ///     // This pointer represents any address, and it may point to anything in memory,
        ///     // allocated or not.
        ///     let ptr1 = kani::any::<usize>() as *const u8;
        ///
        ///     // This pointer address will either point to unallocated memory, to a dead object
        ///     // or to allocated memory within the generator address space.
        ///     let generator = PointerGenerator<u8, 5>::new();
        ///     let ptr2 = generator.any_alloc_status().ptr;
        /// ```
        ///
        /// Kani cannot reason about a pointer allocation status (except for asserting its validity).
        /// Thus, the generator was introduced to help writing harnesses that need to impose
        /// constraints to the arbitrary pointer allocation status.
        /// It also allow us to restrict the pointer provenance, excluding for example address of
        /// variables that are not available in the current context.
        /// As a limitation, it will not cover the entire address space that a pointer can take.
        ///
        /// If your harness do not need to reason about pointer allocation, for example, verifying
        /// pointer wrapping arithmetic, using a pointer with any address will allow you to cover
        /// all possible scenarios.
        #[derive(Debug)]
        pub struct PointerGenerator<T, const BUF_LEN: usize> {
            // Internal allocation that may be used to generate valid pointers.
            buf: MaybeUninit<[T; BUF_LEN]>,
        }

        /// Enumeration with the cases currently covered by the pointer generator.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, crate::kani::Arbitrary)]
        pub enum AllocationStatus {
            /// Dangling pointers
            Dangling,
            /// Pointer to dead object
            DeadObject,
            /// Null pointers
            Null,
            /// In bounds pointer (it may be unaligned)
            InBounds,
            /// Out of bounds
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
                    AllocationStatus::DeadObject => {
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
                    AllocationStatus::OutOfBounds => {
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
