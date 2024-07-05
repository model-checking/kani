// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains functions useful for checking unsafe memory access.
//!
//! Given the following validity rules provided in the Rust documentation:
//! <https://doc.rust-lang.org/std/ptr/index.html> (accessed Feb 6th, 2024)
//!
//! 1. A null pointer is never valid, not even for accesses of size zero.
//! 2. For a pointer to be valid, it is necessary, but not always sufficient, that the pointer
//!    be dereferenceable: the memory range of the given size starting at the pointer must all be
//!    within the bounds of a single allocated object. Note that in Rust, every (stack-allocated)
//!    variable is considered a separate allocated object.
//!    ~~Even for operations of size zero, the pointer must not be pointing to deallocated memory,
//!    i.e., deallocation makes pointers invalid even for zero-sized operations.~~
//!    ZST access is not OK for any pointer.
//!    See: <https://github.com/rust-lang/unsafe-code-guidelines/issues/472>
//! 3. However, casting any non-zero integer literal to a pointer is valid for zero-sized
//!    accesses, even if some memory happens to exist at that address and gets deallocated.
//!    This corresponds to writing your own allocator: allocating zero-sized objects is not very
//!    hard. The canonical way to obtain a pointer that is valid for zero-sized accesses is
//!    `NonNull::dangling`.
//! 4. All accesses performed by functions in this module are non-atomic in the sense of atomic
//!    operations used to synchronize between threads.
//!    This means it is undefined behavior to perform two concurrent accesses to the same location
//!    from different threads unless both accesses only read from memory.
//!    Notice that this explicitly includes `read_volatile` and `write_volatile`:
//!    Volatile accesses cannot be used for inter-thread synchronization.
//! 5. The result of casting a reference to a pointer is valid for as long as the underlying
//!    object is live and no reference (just raw pointers) is used to access the same memory.
//!    That is, reference and pointer accesses cannot be interleaved.
//!
//! Kani is able to verify #1 and #2 today.
//!
//! For #3, we are overly cautious, and Kani will only consider zero-sized pointer access safe if
//! the address matches `NonNull::<()>::dangling()`.
//! The way Kani tracks provenance is not enough to check if the address was the result of a cast
//! from a non-zero integer literal.

#[macro_export]
macro_rules! kani_mem {
    ($core:tt) => {
        use super::kani_intrinsic;
        use private::Internal;
        use $core::mem::{align_of, size_of};
        use $core::ptr::{DynMetadata, NonNull, Pointee};

        /// Check if the pointer is valid for write access according to [crate::mem] conditions 1, 2
        /// and 3.
        ///
        /// Note this function also checks for pointer alignment. Use [self::can_write_unaligned]
        /// if you don't want to fail for unaligned pointers.
        ///
        /// This function does not check if the value stored is valid for the given type. Use
        /// [self::can_dereference] for that.
        ///
        /// This function will panic today if the pointer is not null, and it points to an unallocated or
        /// deallocated memory location. This is an existing Kani limitation.
        /// See <https://github.com/model-checking/kani/issues/2690> for more details.
        // TODO: Add this back! We might need to rename the attribute.
        //#[crate::unstable(
        //    feature = "mem-predicates",
        //    issue = 2690,
        //    reason = "experimental memory predicate API"
        //)]
        pub fn can_write<T>(ptr: *mut T) -> bool
        where
            T: ?Sized,
            <T as Pointee>::Metadata: PtrProperties<T>,
        {
            // The interface takes a mutable pointer to improve readability of the signature.
            // However, using constant pointer avoid unnecessary instrumentation, and it is as powerful.
            // Hence, cast to `*const T`.
            let ptr: *const T = ptr;
            let (thin_ptr, metadata) = ptr.to_raw_parts();
            metadata.is_ptr_aligned(thin_ptr, Internal) && is_inbounds(&metadata, thin_ptr)
        }

        /// Check if the pointer is valid for unaligned write access according to [crate::mem] conditions
        /// 1, 2 and 3.
        ///
        /// Note this function succeeds for unaligned pointers. See [self::can_write] if you also
        /// want to check pointer alignment.
        ///
        /// This function will panic today if the pointer is not null, and it points to an unallocated or
        /// deallocated memory location. This is an existing Kani limitation.
        /// See <https://github.com/model-checking/kani/issues/2690> for more details.
        // TODO: Add this back! We might need to rename the attribute.
        //#[crate::unstable(
        //    feature = "mem-predicates",
        //    issue = 2690,
        //    reason = "experimental memory predicate API"
        //)]
        pub fn can_write_unaligned<T>(ptr: *const T) -> bool
        where
            T: ?Sized,
            <T as Pointee>::Metadata: PtrProperties<T>,
        {
            let (thin_ptr, metadata) = ptr.to_raw_parts();
            is_inbounds(&metadata, thin_ptr)
        }

        /// Checks that pointer `ptr` point to a valid value of type `T`.
        ///
        /// For that, the pointer has to be a valid pointer according to [crate::mem] conditions 1, 2
        /// and 3,
        /// and the value stored must respect the validity invariants for type `T`.
        ///
        /// TODO: Kani should automatically add those checks when a de-reference happens.
        /// <https://github.com/model-checking/kani/issues/2975>
        ///
        /// This function will panic today if the pointer is not null, and it points to an unallocated or
        /// deallocated memory location. This is an existing Kani limitation.
        /// See <https://github.com/model-checking/kani/issues/2690> for more details.
        //#[crate::unstable(
        //    feature = "mem-predicates",
        //    issue = 2690,
        //    reason = "experimental memory predicate API"
        //)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn can_dereference<T>(ptr: *const T) -> bool
        where
            T: ?Sized,
            <T as Pointee>::Metadata: PtrProperties<T>,
        {
            let (thin_ptr, metadata) = ptr.to_raw_parts();
            metadata.is_ptr_aligned(thin_ptr, Internal)
                && is_inbounds(&metadata, thin_ptr)
                && is_initialized(ptr, 1)
                && unsafe { has_valid_value(ptr) }
        }

        /// Checks that pointer `ptr` point to a valid value of type `T`.
        ///
        /// For that, the pointer has to be a valid pointer according to [crate::mem] conditions 1, 2
        /// and 3,
        /// and the value stored must respect the validity invariants for type `T`.
        ///
        /// Note this function succeeds for unaligned pointers. See [self::can_dereference] if you also
        /// want to check pointer alignment.
        ///
        /// This function will panic today if the pointer is not null, and it points to an unallocated or
        /// deallocated memory location. This is an existing Kani limitation.
        /// See <https://github.com/model-checking/kani/issues/2690> for more details.
        // TODO: Add this back! We might need to rename the attribute.
        //#[crate::unstable(
        //    feature = "mem-predicates",
        //    issue = 2690,
        //    reason = "experimental memory predicate API"
        //)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn can_read_unaligned<T>(ptr: *const T) -> bool
        where
            T: ?Sized,
            <T as Pointee>::Metadata: PtrProperties<T>,
        {
            let (thin_ptr, metadata) = ptr.to_raw_parts();
            is_inbounds(&metadata, thin_ptr)
                && is_initialized(ptr, 1)
                && unsafe { has_valid_value(ptr) }
        }

        /// Checks that `data_ptr` points to an allocation that can hold data of size calculated from `T`.
        ///
        /// This will panic if `data_ptr` points to an invalid `non_null`
        fn is_inbounds<M, T>(metadata: &M, data_ptr: *const ()) -> bool
        where
            M: PtrProperties<T>,
            T: ?Sized,
        {
            let sz = metadata.pointee_size(Internal);
            if sz == 0 {
                true // ZST pointers are always valid including nullptr.
            } else if data_ptr.is_null() {
                false
            } else {
                // Note that this branch can't be tested in concrete execution as `is_read_ok` needs to be
                // stubbed.
                // We first assert that the data_ptr
                assert!(
                    unsafe { is_allocated(data_ptr, 0) },
                    "Kani does not support reasoning about pointer to unallocated memory",
                );
                unsafe { is_allocated(data_ptr, sz) }
            }
        }

        mod private {
            /// Define like this to restrict usage of PtrProperties functions outside Kani.
            #[derive(Copy, Clone)]
            pub struct Internal;
        }

        /// Trait that allow us to extract information from pointers without de-referencing them.
        #[doc(hidden)]
        pub trait PtrProperties<T: ?Sized> {
            fn pointee_size(&self, _: Internal) -> usize;

            /// A pointer is aligned if its address is a multiple of its minimum alignment.
            fn is_ptr_aligned(&self, ptr: *const (), internal: Internal) -> bool {
                let min = self.min_alignment(internal);
                ptr as usize % min == 0
            }

            fn min_alignment(&self, _: Internal) -> usize;

            fn dangling(&self, _: Internal) -> *const ();
        }

        /// Get the information for sized types (they don't have metadata).
        impl<T> PtrProperties<T> for () {
            fn pointee_size(&self, _: Internal) -> usize {
                size_of::<T>()
            }

            fn min_alignment(&self, _: Internal) -> usize {
                align_of::<T>()
            }

            fn dangling(&self, _: Internal) -> *const () {
                NonNull::<T>::dangling().as_ptr() as *const _
            }
        }

        /// Get the information from the str metadata.
        impl PtrProperties<str> for usize {
            #[inline(always)]
            fn pointee_size(&self, _: Internal) -> usize {
                *self
            }

            /// String slices are a UTF-8 representation of characters that have the same layout as slices
            /// of type [u8].
            /// <https://doc.rust-lang.org/reference/type-layout.html#str-layout>
            fn min_alignment(&self, _: Internal) -> usize {
                align_of::<u8>()
            }

            fn dangling(&self, _: Internal) -> *const () {
                NonNull::<u8>::dangling().as_ptr() as _
            }
        }

        /// Get the information from the slice metadata.
        impl<T> PtrProperties<[T]> for usize {
            fn pointee_size(&self, _: Internal) -> usize {
                *self * size_of::<T>()
            }

            fn min_alignment(&self, _: Internal) -> usize {
                align_of::<T>()
            }

            fn dangling(&self, _: Internal) -> *const () {
                NonNull::<T>::dangling().as_ptr() as _
            }
        }

        /// Get the information from the vtable.
        impl<T> PtrProperties<T> for DynMetadata<T>
        where
            T: ?Sized,
        {
            fn pointee_size(&self, _: Internal) -> usize {
                self.size_of()
            }

            fn min_alignment(&self, _: Internal) -> usize {
                self.align_of()
            }

            fn dangling(&self, _: Internal) -> *const () {
                NonNull::<&T>::dangling().as_ptr() as _
            }
        }

        /// Check if the pointer `_ptr` contains an allocated address of size equal or greater than `_size`.
        ///
        /// # Safety
        ///
        /// This function should only be called to ensure a pointer is always valid, i.e., in an assertion
        /// context.
        ///
        /// I.e.: This function always returns `true` if the pointer is valid.
        /// Otherwise, it returns non-det boolean.
        #[rustc_diagnostic_item = "KaniIsAllocated"]
        #[inline(never)]
        unsafe fn is_allocated(_ptr: *const (), _size: usize) -> bool {
            kani_intrinsic()
        }

        /// Check if the value stored in the given location satisfies type `T` validity requirements.
        ///
        /// # Safety
        ///
        /// - Users have to ensure that the pointer is aligned the pointed memory is allocated.
        #[rustc_diagnostic_item = "KaniValidValue"]
        #[inline(never)]
        unsafe fn has_valid_value<T: ?Sized>(_ptr: *const T) -> bool {
            kani_intrinsic()
        }

        /// Check whether `len * size_of::<T>()` bytes are initialized starting from `ptr`.
        #[rustc_diagnostic_item = "KaniIsInitialized"]
        #[inline(never)]
        pub(crate) fn is_initialized<T: ?Sized>(_ptr: *const T, _len: usize) -> bool {
            kani_intrinsic()
        }

        /// Get the object ID of the given pointer.
        #[rustc_diagnostic_item = "KaniPointerObject"]
        #[inline(never)]
        pub(crate) fn pointer_object<T: ?Sized>(_ptr: *const T) -> usize {
            kani_intrinsic()
        }

        /// Get the object offset of the given pointer.
        #[rustc_diagnostic_item = "KaniPointerOffset"]
        #[inline(never)]
        pub(crate) fn pointer_offset<T: ?Sized>(_ptr: *const T) -> usize {
            kani_intrinsic()
        }
    };
}
