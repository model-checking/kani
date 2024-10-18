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
            #[kanitool::fn_marker = "SizeOfValRawModel"]
            pub fn size_of_val_raw<T: ?Sized>(ptr: *const T) -> usize {
                if let Some(size) = kani::mem::checked_size_of_raw(ptr) {
                    size
                } else {
                    kani::safety_check(false, "failed to compute size of val");
                    // Unreachable without panic.
                    kani::kani_intrinsic()
                }
            }

            #[kanitool::fn_marker = "AlignOfValRawModel"]
            pub fn align_of_val_raw<T: ?Sized>(ptr: *const T) -> usize {
                if let Some(size) = kani::mem::checked_align_of_raw(ptr) {
                    size
                } else {
                    kani::safety_check(false, "failed to compute align of val");
                    // Unreachable without panic.
                    kani::kani_intrinsic()
                }
            }
        }

        #[allow(dead_code)]
        mod mem_models {
            use core::ptr::{self, DynMetadata, Pointee};

            /// Retrieve the size of the object stored in the vtable.
            ///
            /// This model is used to implement `size_of_unsized_portion` intrinsic.
            ///
            /// For that, `U` is a trait, and `T` is either equal to `U` or has a tail `U`.
            #[kanitool::fn_marker = "SizeOfDynPortionModel"]
            pub(crate) fn size_of_dyn_portion<T, U: ?Sized>(ptr: *const T) -> Option<usize>
            where
                T: ?Sized + Pointee<Metadata = DynMetadata<U>>,
            {
                Some(ptr::metadata(ptr).size_of())
            }

            /// Retrieve the alignment of the object stored in the vtable.
            ///
            /// This model is used to implement `align_of_raw` intrinsic.
            ///
            /// For that, `U` is a trait, and `T` is either equal to `U` or has a tail `U`.
            #[kanitool::fn_marker = "AlignOfDynPortionModel"]
            pub(crate) fn align_of_dyn_portion<T, U: ?Sized>(
                ptr: *const T,
                sized_portion: usize,
            ) -> Option<usize>
            where
                T: ?Sized + Pointee<Metadata = DynMetadata<U>>,
            {
                Some(ptr::metadata(ptr).align_of().max(sized_portion))
            }
        }
    };
}
