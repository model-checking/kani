// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Contains definitions that Kani compiler may use to model functions that are not suitable for
//! verification or functions without a body, such as intrinsics.
//!
//! Note that these are models that Kani uses by default; thus, we keep them separate from stubs.

// Definitions in this module are not meant to be visible to the end user, only the compiler.
#[allow(dead_code)]
mod intrinsics {
    use std::fmt::Debug;
    use std::mem::size_of;

    /// Similar definition to portable SIMD.
    /// We cannot reuse theirs since TRUE and FALSE defs are private.
    /// We leave this private today, since this is not necessarily a final solution, so we don't
    /// want users relying on this.
    /// Our definitions are also a bit more permissive to comply with the platform intrinsics.
    pub(super) trait MaskElement: PartialEq + Debug {
        const TRUE: Self;
        const FALSE: Self;
    }

    macro_rules! impl_element {
        { $ty:ty } => {
            impl MaskElement for $ty {
                const TRUE: Self = -1;
                const FALSE: Self = 0;
            }
        }
    }

    macro_rules! impl_unsigned_element {
        { $ty:ty } => {
            impl MaskElement for $ty {
                // Note that in the declaration of the intrinsic it is documented that the lane
                // values should be -1 or 0:
                // <https://github.com/rust-lang/rust/blob/338cfd3/library/portable-simd/crates/core_simd/src/intrinsics.rs#L134-L144>
                //
                // However, MIRI and the Rust compiler seems to accept unsigned values and they
                // use their binary representation. Thus, that's what we use for now.
                /// All bits are 1 which represents TRUE.
                const TRUE: Self = <$ty>::MAX;
                /// All bits are 0 which represents FALSE.
                const FALSE: Self = 0;
            }
        }
    }

    impl_element! { i8 }
    impl_element! { i16 }
    impl_element! { i32 }
    impl_element! { i64 }
    impl_element! { i128 }
    impl_element! { isize }

    impl_unsigned_element! { u8 }
    impl_unsigned_element! { u16 }
    impl_unsigned_element! { u32 }
    impl_unsigned_element! { u64 }
    impl_unsigned_element! { u128 }
    impl_unsigned_element! { usize }

    /// Calculate the minimum number of lanes to represent a mask
    /// Logic similar to `bitmask_len` from `portable_simd`.
    /// <https://github.com/rust-lang/portable-simd/blob/490b5cf/crates/core_simd/src/masks/to_bitmask.rs#L75-L79>
    pub(super) const fn mask_len(len: usize) -> usize {
        (len + 7) / 8
    }

    #[cfg(target_endian = "little")]
    unsafe fn simd_bitmask_impl<T, const LANES: usize>(input: &[T; LANES]) -> [u8; mask_len(LANES)]
    where
        T: MaskElement,
    {
        let mut mask_array = [0; mask_len(LANES)];
        for lane in (0..input.len()).rev() {
            let byte = lane / 8;
            let mask = &mut mask_array[byte];
            let shift_mask = *mask << 1;
            *mask = if input[lane] == T::TRUE {
                shift_mask | 0x1
            } else {
                assert_eq!(input[lane], T::FALSE, "Masks values should either be 0 or -1");
                shift_mask
            };
        }
        mask_array
    }

    /// Stub for simd_bitmask.
    ///
    /// It will reduce a simd vector (TxN), into an integer of size S (in bits), where S >= N.
    /// Each bit of the output will represent a lane from the input. A lane value of all 0's will be
    /// translated to 1b0, while all 1's will be translated to 1b1.
    ///
    /// In order to be able to do this pragmatically, we take additional parameters that are filled
    /// by the compiler.
    #[rustc_diagnostic_item = "KaniModelSimdBitmask"]
    pub(super) unsafe fn simd_bitmask<T, U, E, const LANES: usize>(input: T) -> U
    where
        [u8; mask_len(LANES)]: Sized,
        E: MaskElement,
    {
        // These checks are compiler sanity checks to ensure we are not doing anything invalid.
        assert_eq!(
            size_of::<U>(),
            size_of::<[u8; mask_len(LANES)]>(),
            "Expected size of return type and mask lanes to match",
        );
        assert_eq!(
            size_of::<T>(),
            size_of::<Simd::<E, LANES>>(),
            "Expected size of input and lanes to match",
        );

        let data = &*(&input as *const T as *const [E; LANES]);
        let mask = simd_bitmask_impl(data);
        (&mask as *const [u8; mask_len(LANES)] as *const U).read()
    }

    /// Structure used for sanity check our parameters.
    #[repr(simd)]
    struct Simd<T, const LANES: usize>([T; LANES]);
}

#[cfg(test)]
mod test {
    use super::intrinsics as kani_intrinsic;
    use std::{fmt::Debug, simd::*};

    extern "platform-intrinsic" {
        fn simd_bitmask<T, U>(x: T) -> U;
    }

    /// Test that the `simd_bitmask` model is equivalent to the intrinsic for all true and all false
    /// masks with lanes represented using i16.
    #[test]
    fn test_bitmask_i16() {
        check_portable_bitmask::<_, i16, 16>(mask16x16::splat(false));
        check_portable_bitmask::<_, i16, 16>(mask16x16::splat(true));
    }

    /// Tests that the model correctly fails if an invalid value is given.
    #[test]
    #[should_panic(expected = "Masks values should either be 0 or -1")]
    fn test_invalid_bitmask() {
        let invalid_mask = unsafe { mask32x16::from_int_unchecked(i32x16::splat(10)) };
        assert_eq!(
            unsafe { kani_intrinsic::simd_bitmask::<_, u16, i32, 16>(invalid_mask) },
            u16::MAX
        );
    }

    /// Tests that the model correctly fails if the size parameter of the mask doesn't match the
    /// expected number of bytes in the representation.
    #[test]
    #[should_panic(expected = "Expected size of return type and mask lanes to match")]
    fn test_invalid_generics() {
        let mask = mask32x16::splat(false);
        assert_eq!(unsafe { kani_intrinsic::simd_bitmask::<_, u16, i32, 2>(mask) }, u16::MAX);
    }

    /// Test that the `simd_bitmask` model is equivalent to the intrinsic for a few random values.
    /// These values shouldn't be symmetric and ensure that we also handle endianness correctly.
    #[test]
    fn test_bitmask_i32() {
        check_portable_bitmask::<_, i32, 8>(mask32x8::from([
            true, true, false, true, false, false, false, true,
        ]));

        check_portable_bitmask::<_, i32, 4>(mask32x4::from([true, false, false, true]));
    }

    #[repr(simd)]
    #[derive(Clone, Debug)]
    struct CustomMask<T, const LANES: usize>([T; LANES]);

    /// Check that the bitmask model can handle odd size SIMD arrays.
    /// Since the portable_simd restricts the number of lanes, we have to use our own custom SIMD.
    #[test]
    fn test_bitmask_odd_lanes() {
        check_bitmask::<_, [u8; 3], i128, 23>(CustomMask([0i128; 23]));
        check_bitmask::<_, [u8; 9], i128, 70>(CustomMask([-1i128; 70]));
    }

    /// Compare the value returned by our model and the portable simd representation.
    fn check_portable_bitmask<T, E, const LANES: usize>(mask: Mask<T, LANES>)
    where
        T: std::simd::MaskElement,
        LaneCount<LANES>: SupportedLaneCount,
        E: kani_intrinsic::MaskElement,
        [u8; kani_intrinsic::mask_len(LANES)]: Sized,
    {
        assert_eq!(
            unsafe { kani_intrinsic::simd_bitmask::<_, u64, E, LANES>(mask.clone()) },
            mask.to_bitmask()
        );
    }

    /// Compare the value returned by our model and the simd_bitmask intrinsic.
    fn check_bitmask<T, U, E, const LANES: usize>(mask: T)
    where
        T: Clone,
        U: PartialEq + Debug,
        E: kani_intrinsic::MaskElement,
        [u8; kani_intrinsic::mask_len(LANES)]: Sized,
    {
        assert_eq!(
            unsafe { kani_intrinsic::simd_bitmask::<_, U, E, LANES>(mask.clone()) },
            unsafe { simd_bitmask::<T, U>(mask) }
        );
    }
}
