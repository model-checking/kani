// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Contains definitions that Kani compiler may use to model functions that are not suitable for
//! verification or functions without a body, such as intrinsics.
//!
//! Note that these are models that Kani uses by default; thus, we keep them separate from stubs.
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
                    // Note that today we trigger a safety check for foreign types.
                    kani::safety_check(false, "failed to compute `align_of_val`");
                    // Unreachable without panic.
                    kani::kani_intrinsic()
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
                    // Adjust size to be a multiple of alignment, i.e.: (size + (align - 1)) & -align
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
            /// Most information are extracted by the Kani compiler at compilation time,
            /// except for the length of the slice.
            /// Thus, alignment should be safe to use (power-of-two and smaller than isize::MAX).
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
                // Adjust size to be a multiple of alignment, i.e.: (size + (align - 1)) & -align
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
