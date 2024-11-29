// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains functions useful for float-related checks

#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! generate_float {
    ($core:path) => {
        use super::kani_intrinsic;
        use core::convert::FloatToInt;
        /// Returns whether the given float `value` satisfies the range
        /// condition of the `to_int_unchecked` methods, namely that the `value`
        /// after truncation is in range of the target `Int`
        ///
        /// # Example:
        ///
        /// ```no_run
        /// let f: f32 = 145.7;
        /// let fits_in_i8 = kani::float::float_to_int_in_range::<f32, i8>(f);
        /// // doesn't fit in `i8` because the value after truncation (`145.0`) is larger than `i8::MAX`
        /// assert!(!fits_in_i8);
        ///
        /// let f: f64 = 1e6;
        /// let fits_in_u32 = kani::float::float_to_int_in_range::<f64, u32>(f);
        /// // fits in `u32` because the value after truncation (`1e6`) is smaller than `u32::MAX`
        /// assert!(fits_in_u32);
        /// ```
        #[crate::kani::unstable_feature(
            feature = "float-lib",
            issue = "none",
            reason = "experimental floating-point API"
        )]
        #[kanitool::fn_marker = "FloatToIntInRangeHook"]
        #[inline(never)]
        pub fn float_to_int_in_range<Float, Int>(value: Float) -> bool
        where
            Float: FloatToInt<Int>,
        {
            kani_intrinsic()
        }
    };
}
