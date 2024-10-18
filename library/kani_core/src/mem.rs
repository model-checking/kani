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
//!
// TODO: This module is currently tightly coupled with CBMC's memory model, and it needs some
//       refactoring to be used with other backends.

#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! kani_mem {
    ($core:tt) => {
        use super::kani_intrinsic;
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
        #[crate::kani::unstable_feature(
            feature = "mem-predicates",
            issue = 2690,
            reason = "experimental memory predicate API"
        )]
        pub fn can_write<T: ?Sized>(ptr: *mut T) -> bool {
            is_ptr_aligned(ptr) && is_inbounds(ptr)
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
        #[crate::kani::unstable_feature(
            feature = "mem-predicates",
            issue = 2690,
            reason = "experimental memory predicate API"
        )]
        pub fn can_write_unaligned<T: ?Sized>(ptr: *const T) -> bool {
            let (thin_ptr, metadata) = ptr.to_raw_parts();
            is_inbounds(ptr)
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
        #[crate::kani::unstable_feature(
            feature = "mem-predicates",
            issue = 2690,
            reason = "experimental memory predicate API"
        )]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn can_dereference<T: ?Sized>(ptr: *const T) -> bool {
            // Need to assert `is_initialized` because non-determinism is used under the hood, so it
            // does not make sense to use it inside assumption context.
            is_ptr_aligned(ptr)
                && is_inbounds(ptr)
                && assert_is_initialized(ptr)
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
        #[crate::kani::unstable_feature(
            feature = "mem-predicates",
            issue = 2690,
            reason = "experimental memory predicate API"
        )]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn can_read_unaligned<T: ?Sized>(ptr: *const T) -> bool {
            let (thin_ptr, metadata) = ptr.to_raw_parts();
            // Need to assert `is_initialized` because non-determinism is used under the hood, so it
            // does not make sense to use it inside assumption context.
            is_inbounds(ptr) && assert_is_initialized(ptr) && unsafe { has_valid_value(ptr) }
        }

        /// Check if two pointers points to the same allocated object, and that both pointers
        /// are in bounds of that object.
        ///
        /// A pointer is still considered in-bounds if it points to 1-byte past the allocation.
        #[crate::kani::unstable_feature(
            feature = "mem-predicates",
            issue = 2690,
            reason = "experimental memory predicate API"
        )]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub fn same_allocation<T>(ptr1: *const T, ptr2: *const T) -> bool {
            cbmc::same_allocation(ptr1, ptr2)
        }

        /// Compute the size of the val pointed to if safe.
        ///
        /// Return `None` if an overflow would occur, or if alignment is not power of two.
        /// TODO: Optimize this if T is sized.
        pub fn checked_size_of_raw<T: ?Sized>(ptr: *const T) -> Option<usize> {
            #[cfg(not(feature = "concrete_playback"))]
            {
                let size_of_unsized = crate::kani::size_of_unsized_portion(ptr)?;
                let sum = size_of_unsized.checked_add(crate::kani::size_of_sized_portion::<T>())?;
                let align = checked_align_of_raw(ptr)?;
                // Size must be multiple of alignment.
                // Since alignment is power-of-two, we can compute as (size + (align - 1)) & -align
                return Some((sum.checked_add(align - 1))? & align.wrapping_neg());
            }

            #[cfg(feature = "concrete_playback")]
            if core::mem::size_of::<<T as Pointee>::Metadata>() == 0 {
                // SAFETY: It is currently safe to call this with a thin pointer.
                unsafe { Some(core::mem::size_of_val_raw(ptr)) }
            } else {
                panic!("Cannot safely compute size of `{}` at runtime", core::any::type_name::<T>())
            }
        }

        /// Compute the size of the val pointed to if safe.
        ///
        /// Return `None` if alignment information cannot be retrieved (foreign types), or if value
        /// is not power-of-two.
        pub fn checked_align_of_raw<T: ?Sized>(ptr: *const T) -> Option<usize> {
            crate::kani::align_of_raw(ptr)
                .and_then(|align| align.is_power_of_two().then_some(align))
        }

        /// Checks that `ptr` points to an allocation that can hold data of size calculated from `T`.
        ///
        /// This will panic if `ptr` points to an invalid `non_null`
        fn is_inbounds<T: ?Sized>(ptr: *const T) -> bool {
            // If size overflows, then pointer cannot be inbounds.
            let Some(sz) = checked_size_of_raw(ptr) else { return false };
            if sz == 0 {
                true // ZST pointers are always valid including nullptr.
            } else if ptr.is_null() {
                false
            } else {
                // Note that this branch can't be tested in concrete execution as `is_read_ok` needs to be
                // stubbed.
                // We first assert that the data_ptr
                let data_ptr = ptr as *const ();
                super::assert(
                    unsafe { is_allocated(data_ptr, 0) },
                    "Kani does not support reasoning about pointer to unallocated memory",
                );
                unsafe { is_allocated(data_ptr, sz) }
            }
        }

        // Return whether the pointer is aligned
        #[allow(clippy::manual_is_power_of_two)]
        fn is_ptr_aligned<T: ?Sized>(ptr: *const T) -> bool {
            // Cannot be aligned if pointer alignment cannot be computed.
            let Some(align) = checked_align_of_raw(ptr) else { return false };
            if align > 0 && (align & (align - 1)) == 0 {
                // Mod of power of 2 can be done with an &.
                ptr as *const () as usize & (align - 1) == 0
            } else {
                // Alignment is not a valid value (not a power of two).
                false
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
        #[kanitool::fn_marker = "ValidValueIntrinsic"]
        #[inline(never)]
        unsafe fn has_valid_value<T: ?Sized>(_ptr: *const T) -> bool {
            kani_intrinsic()
        }

        /// Check whether `len * size_of::<T>()` bytes are initialized starting from `ptr`.
        #[kanitool::fn_marker = "IsInitializedIntrinsic"]
        #[inline(never)]
        pub(crate) fn is_initialized<T: ?Sized>(_ptr: *const T) -> bool {
            kani_intrinsic()
        }

        /// A helper to assert `is_initialized` to use it as a part of other predicates.
        fn assert_is_initialized<T: ?Sized>(ptr: *const T) -> bool {
            super::check(
                is_initialized(ptr),
                "Undefined Behavior: Reading from an uninitialized pointer",
            );
            true
        }

        pub(super) mod cbmc {
            use super::*;
            /// CBMC specific implementation of [super::same_allocation].
            pub fn same_allocation<T>(ptr1: *const T, ptr2: *const T) -> bool {
                let obj1 = crate::kani::mem::pointer_object(ptr1);
                (obj1 == crate::kani::mem::pointer_object(ptr2)) && {
                    crate::kani::assert(
                        unsafe {
                            is_allocated(ptr1 as *const (), 0) || is_allocated(ptr2 as *const (), 0)
                        },
                        "Kani does not support reasoning about pointer to unallocated memory",
                    );
                    unsafe {
                        is_allocated(ptr1 as *const (), 0) && is_allocated(ptr2 as *const (), 0)
                    }
                }
            }
        }

        /// Get the object ID of the given pointer.
        #[doc(hidden)]
        #[rustc_diagnostic_item = "KaniPointerObject"]
        #[inline(never)]
        pub(crate) fn pointer_object<T: ?Sized>(_ptr: *const T) -> usize {
            kani_intrinsic()
        }

        /// Get the object offset of the given pointer.
        #[doc(hidden)]
        #[rustc_diagnostic_item = "KaniPointerOffset"]
        #[inline(never)]
        pub(crate) fn pointer_offset<T: ?Sized>(_ptr: *const T) -> usize {
            kani_intrinsic()
        }
    };
}
