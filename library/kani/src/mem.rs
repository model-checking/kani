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
//!    Even for operations of size zero, the pointer must not be pointing to deallocated memory,
//!    i.e., deallocation makes pointers invalid even for zero-sized operations.
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

use crate::kani_intrinsic;
use crate::mem::private::Internal;
use std::mem::{align_of, size_of};
use std::ptr::{DynMetadata, NonNull, Pointee};

/// Assert that the pointer is valid for access according to [crate::mem] conditions 1, 2 and 3.
///
/// Note that an unaligned pointer is still considered valid.
///
/// TODO: Kani should automatically add those checks when a de-reference happens.
/// <https://github.com/model-checking/kani/issues/2975>
///
/// This function will either panic or return `true`. This is to make it easier to use it in
/// contracts.
#[crate::unstable(
    feature = "mem-predicates",
    issue = 2690,
    reason = "experimental memory predicate API"
)]
pub fn assert_valid_ptr<T>(ptr: *const T) -> bool
where
    T: ?Sized,
    <T as Pointee>::Metadata: PtrProperties<T>,
{
    crate::assert(!ptr.is_null(), "Expected valid pointer, but found `null`");

    let (thin_ptr, metadata) = ptr.to_raw_parts();
    let sz = metadata.pointee_size(Internal);
    if sz == 0 {
        true // ZST pointers are always valid
    } else {
        // Note that this branch can't be tested in concrete execution as `is_read_ok` needs to be
        // stubbed.
        crate::assert(
            is_read_ok(thin_ptr, sz),
            "Expected valid pointer, but found dangling pointer",
        );
        true
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
/// This function should only be called to ensure a pointer is valid. The opposite isn't true.
/// I.e.: This function always returns `true` if the pointer is valid.
/// Otherwise, it returns non-det boolean.
#[rustc_diagnostic_item = "KaniIsReadOk"]
#[inline(never)]
fn is_read_ok(_ptr: *const (), _size: usize) -> bool {
    kani_intrinsic()
}

#[cfg(test)]
mod tests {
    use super::{assert_valid_ptr, PtrProperties};
    use crate::mem::private::Internal;
    use std::fmt::Debug;
    use std::intrinsics::size_of;
    use std::mem::{align_of, align_of_val, size_of_val};
    use std::ptr;
    use std::ptr::{NonNull, Pointee};

    fn size_of_t<T>(ptr: *const T) -> usize
    where
        T: ?Sized,
        <T as Pointee>::Metadata: PtrProperties<T>,
    {
        let (_, metadata) = ptr.to_raw_parts();
        metadata.pointee_size(Internal)
    }

    fn align_of_t<T>(ptr: *const T) -> usize
    where
        T: ?Sized,
        <T as Pointee>::Metadata: PtrProperties<T>,
    {
        let (_, metadata) = ptr.to_raw_parts();
        metadata.min_alignment(Internal)
    }

    #[test]
    fn test_size_of() {
        assert_eq!(size_of_t("hi"), size_of_val("hi"));
        assert_eq!(size_of_t(&0u8), size_of_val(&0u8));
        assert_eq!(size_of_t(&0u8 as *const dyn std::fmt::Display), size_of_val(&0u8));
        assert_eq!(size_of_t(&[0u8, 1u8] as &[u8]), size_of_val(&[0u8, 1u8]));
        assert_eq!(size_of_t(&[] as &[u8]), size_of_val::<[u8; 0]>(&[]));
        assert_eq!(
            size_of_t(NonNull::<u32>::dangling().as_ptr() as *const dyn std::fmt::Display),
            size_of::<u32>()
        );
    }

    #[test]
    fn test_alignment() {
        assert_eq!(align_of_t("hi"), align_of_val("hi"));
        assert_eq!(align_of_t(&0u8), align_of_val(&0u8));
        assert_eq!(align_of_t(&0u32 as *const dyn std::fmt::Display), align_of_val(&0u32));
        assert_eq!(align_of_t(&[0isize, 1isize] as &[isize]), align_of_val(&[0isize, 1isize]));
        assert_eq!(align_of_t(&[] as &[u8]), align_of_val::<[u8; 0]>(&[]));
        assert_eq!(
            align_of_t(NonNull::<u32>::dangling().as_ptr() as *const dyn std::fmt::Display),
            align_of::<u32>()
        );
    }

    #[test]
    pub fn test_empty_slice() {
        let slice_ptr = Vec::<char>::new().as_slice() as *const [char];
        assert_valid_ptr(slice_ptr);
    }

    #[test]
    pub fn test_empty_str() {
        let slice_ptr = String::new().as_str() as *const str;
        assert_valid_ptr(slice_ptr);
    }

    #[test]
    fn test_dangling_zst() {
        test_dangling_of_t::<()>();
        test_dangling_of_t::<[(); 10]>();
    }

    fn test_dangling_of_t<T>() {
        let dangling: *const T = NonNull::<T>::dangling().as_ptr();
        assert_valid_ptr(dangling);

        let vec_ptr = Vec::<T>::new().as_ptr();
        assert_valid_ptr(vec_ptr);
    }

    #[test]
    #[should_panic(expected = "Expected valid pointer, but found `null`")]
    fn test_null_fat_ptr() {
        assert_valid_ptr(ptr::null::<char>() as *const dyn Debug);
    }

    #[test]
    #[should_panic(expected = "Expected valid pointer, but found `null`")]
    fn test_null_char() {
        assert_valid_ptr(ptr::null::<char>());
    }
}
