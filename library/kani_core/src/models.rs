// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Contains definitions that Kani compiler may use to model functions that are not suitable for
//! verification or functions without a body, such as intrinsics.
//!
//! Note that these are models that Kani uses by default, and they should not be user visible.
//! Thus, we separate them from stubs.
//! TODO: Move SIMD model here.

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_models {
    () => {
        /// Model rustc intrinsics. These definitions are not visible to the crate user.
        /// They are used by Kani's compiler.
        #[allow(dead_code)]
        mod rustc_intrinsics {
            use crate::kani;
            use core::convert::TryFrom;
            use core::ptr::Pointee;

            #[kanitool::fn_marker = "SizeOfValRawModel"]
            pub fn size_of_val_raw<T: ?Sized>(ptr: *const T) -> usize {
                if let Some(size) = kani::mem::checked_size_of_raw(ptr) {
                    size
                } else if core::mem::size_of::<<T as Pointee>::Metadata>() == 0 {
                    kani::panic("cannot compute `size_of_val` for extern types")
                } else {
                    kani::safety_check(false, "failed to compute `size_of_val`");
                    // Unreachable without panic.
                    kani::kani_intrinsic()
                }
            }

            #[kanitool::fn_marker = "AlignOfValRawModel"]
            pub fn align_of_val_raw<T: ?Sized>(ptr: *const T) -> usize {
                if let Some(size) = kani::mem::checked_align_of_raw(ptr) {
                    size
                } else if core::mem::size_of::<<T as Pointee>::Metadata>() == 0 {
                    kani::panic("cannot compute `align_of_val` for extern types")
                } else {
                    kani::safety_check(false, "failed to compute `align_of_val`");
                    // Unreachable without panic.
                    kani::kani_intrinsic()
                }
            }

            /// Implements core::intrinsics::ptr_offfset_from with safety checks in place.
            ///
            /// From original documentation:
            ///
            /// # Safety
            ///
            /// If any of the following conditions are violated, the result is Undefined Behavior:
            ///
            /// * `self` and `origin` must either
            ///
            ///   * point to the same address, or
            ///   * both be *derived from* a pointer to the same allocated object,
            ///     and the memory range between
            ///     the two pointers must be in bounds of that object.
            ///
            /// * The distance between the pointers, in bytes, must be an exact multiple
            ///   of the size of `T`.
            ///
            /// # Panics
            ///
            /// This function panics if `T` is a Zero-Sized Type ("ZST").
            #[kanitool::fn_marker = "PtrOffsetFromModel"]
            pub unsafe fn ptr_offset_from<T>(ptr1: *const T, ptr2: *const T) -> isize {
                // This is not a safety condition.
                kani::assert(core::mem::size_of::<T>() > 0, "Cannot compute offset of a ZST");
                if ptr1 == ptr2 {
                    0
                } else {
                    kani::safety_check(
                        kani::mem::same_allocation_internal(ptr1, ptr2),
                        "Offset result and original pointer should point to the same allocation",
                    );
                    // The offset must fit in isize since this represents the same allocation.
                    let offset_bytes = ptr1.addr().wrapping_sub(ptr2.addr()) as isize;
                    let t_size = size_of::<T>() as isize;
                    kani::safety_check(
                        offset_bytes % t_size == 0,
                        "Expected the distance between the pointers, in bytes, to be a
                        multiple of the size of `T`",
                    );
                    offset_bytes / t_size
                }
            }

            #[kanitool::fn_marker = "PtrSubPtrModel"]
            pub unsafe fn ptr_sub_ptr<T>(ptr1: *const T, ptr2: *const T) -> usize {
                let offset = ptr_offset_from(ptr1, ptr2);
                kani::safety_check(offset >= 0, "Expected non-negative distance between pointers");
                offset as usize
            }

            /// An offset model that checks UB.
            #[kanitool::fn_marker = "OffsetModel"]
            pub fn offset<T, P: Ptr<T>, O: ToISize>(ptr: P, offset: O) -> P {
                let offset = offset.to_isize();
                let t_size = core::mem::size_of::<T>() as isize;
                if offset == 0 || t_size == 0 {
                    // It's always safe to perform an offset of length 0.
                    ptr
                } else {
                    let (byte_offset, overflow) = offset.overflowing_mul(t_size);
                    kani::safety_check(!overflow, "Offset in bytes overflows isize");
                    let orig_ptr = ptr.to_const_ptr();
                    // NOTE: For CBMC, using the pointer addition can have unexpected behavior
                    // when the offset is higher than the object bits since it will wrap around.
                    // See for more details: https://github.com/model-checking/kani/issues/1150
                    //
                    // However, when I tried implementing this using usize operation, we got some
                    // unexpected failures that still require further debugging.
                    // let new_ptr = orig_ptr.addr().wrapping_add_signed(byte_offset) as *const T;
                    let new_ptr = orig_ptr.wrapping_byte_offset(byte_offset);
                    kani::safety_check(
                        kani::mem::same_allocation_internal(orig_ptr, new_ptr),
                        "Offset result and original pointer must point to the same allocation",
                    );
                    P::from_const_ptr(new_ptr)
                }
            }

            pub trait Ptr<T> {
                fn to_const_ptr(self) -> *const T;
                fn from_const_ptr(ptr: *const T) -> Self;
            }

            impl<T> Ptr<T> for *const T {
                fn to_const_ptr(self) -> *const T {
                    self
                }
                fn from_const_ptr(ptr: *const T) -> Self {
                    ptr
                }
            }

            impl<T> Ptr<T> for *mut T {
                fn to_const_ptr(self) -> *const T {
                    self
                }
                fn from_const_ptr(ptr: *const T) -> Self {
                    ptr as _
                }
            }

            pub trait ToISize {
                fn to_isize(self) -> isize;
            }

            impl ToISize for isize {
                fn to_isize(self) -> isize {
                    self
                }
            }

            impl ToISize for usize {
                fn to_isize(self) -> isize {
                    if let Ok(val) = self.try_into() {
                        val
                    } else {
                        kani::safety_check(false, "Offset value overflows isize");
                        unreachable!();
                    }
                }
            }
        }

        #[allow(dead_code)]
        mod mem_models {
            use core::ptr::{self, DynMetadata, Pointee};

            /// Retrieve the size of the object pointed by the given raw pointer.
            ///
            /// Where `U` is a trait, and `T` is either equal to `U` or has a tail `U`.
            ///
            /// In cases where `T` is different than `U`,
            /// `T` may have a sized portion, the head, while the unsized portion will be at its
            /// tail.
            ///
            /// Arguments `head_size` and `head_align` represent the size and alignment of the sized
            /// portion.
            /// These values are known at compilation time, and they are extracted by the compiler.
            /// If `T` doesn't have a sized portion, or if `T` is equal to `U`,
            /// `head_size` will be set to `0`, and `head_align` will be set to 1.
            ///
            /// This model is used to implement `checked_size_of_raw`.
            #[kanitool::fn_marker = "SizeOfDynObjectModel"]
            pub(crate) fn size_of_dyn_object<T, U: ?Sized>(
                ptr: *const T,
                head_size: usize,
                head_align: usize,
            ) -> Option<usize>
            where
                T: ?Sized + Pointee<Metadata = DynMetadata<U>>,
            {
                let metadata = ptr::metadata(ptr);
                let align = metadata.align_of().max(head_align);
                if align.is_power_of_two() {
                    let size_dyn = metadata.size_of();
                    let (total, sum_overflow) = size_dyn.overflowing_add(head_size);
                    // Round up size to the nearest multiple of alignment, i.e.: (size + (align - 1)) & -align
                    let (adjust, adjust_overflow) = total.overflowing_add(align.wrapping_sub(1));
                    let adjusted_size = adjust & align.wrapping_neg();
                    if sum_overflow || adjust_overflow || adjusted_size > isize::MAX as _ {
                        None
                    } else {
                        Some(adjusted_size)
                    }
                } else {
                    None
                }
            }

            /// Retrieve the alignment of the object stored in the vtable.
            ///
            /// Where `U` is a trait, and `T` is either equal to `U` or has a tail `U`.
            ///
            /// In cases where `T` is different than `U`,
            /// `T` may have a sized portion, the head, while the unsized portion will be at its
            /// tail.
            ///
            /// `head_align` represents the alignment of the sized portion,
            /// and its value is known at compilation time.
            ///
            /// If `T` doesn't have a sized portion, or if `T` is equal to `U`,
            /// `head_align` will be set to 1.
            ///
            /// This model is used to implement `checked_aligned_of_raw`.
            #[kanitool::fn_marker = "AlignOfDynObjectModel"]
            pub(crate) fn align_of_dyn_object<T, U: ?Sized>(
                ptr: *const T,
                head_align: usize,
            ) -> Option<usize>
            where
                T: ?Sized + Pointee<Metadata = DynMetadata<U>>,
            {
                let align = ptr::metadata(ptr).align_of().max(head_align);
                align.is_power_of_two().then_some(align)
            }

            /// Compute the size of a slice or object with a slice tail.
            ///
            /// The slice length may be a symbolic value which is computed at runtime.
            /// All the other inputs are extracted and validated by Kani compiler,
            /// i.e., these are well known concrete values that should be safe to use.
            /// Example, align is a power-of-two and smaller than isize::MAX.
            ///
            /// Thus, this generate the logic to ensure the size computation does not
            /// does not overflow and it is smaller than `isize::MAX`.
            #[kanitool::fn_marker = "SizeOfSliceObjectModel"]
            pub(crate) fn size_of_slice_object(
                len: usize,
                elem_size: usize,
                head_size: usize,
                align: usize,
            ) -> Option<usize> {
                let (slice_sz, mul_overflow) = elem_size.overflowing_mul(len);
                let (total, sum_overflow) = slice_sz.overflowing_add(head_size);
                // Round up size to the nearest multiple of alignment, i.e.: (size + (align - 1)) & -align
                let (adjust, adjust_overflow) = total.overflowing_add(align.wrapping_sub(1));
                let adjusted_size = adjust & align.wrapping_neg();
                if mul_overflow
                    || sum_overflow
                    || adjust_overflow
                    || adjusted_size > isize::MAX as _
                {
                    None
                } else {
                    Some(adjusted_size)
                }
            }
        }
    };
}
