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
