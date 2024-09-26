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
