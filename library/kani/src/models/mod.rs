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
        len.div_ceil(8)
    }

    #[cfg(target_endian = "little")]
    unsafe fn simd_bitmask_impl<T, const LANES: usize>(input: &[T; LANES]) -> [u8; mask_len(LANES)]
    where
        T: MaskElement,
    {
        let mut mask_array = [0; mask_len(LANES)];

        // Process 8 lanes at a time when possible
        for (byte_idx, byte) in mask_array.iter_mut().enumerate() {
            // Calculate the starting lane for this byte
            let start_lane = byte_idx * 8;
            // Calculate how many bits to process (handle the last byte which might be partial)
            let bits_to_process = (LANES - start_lane).min(8);

            *byte = if bits_to_process > 0 && input[start_lane] == T::TRUE { 1 << 0 } else { 0 }
                | if bits_to_process > 1 && input[start_lane + 1] == T::TRUE { 1 << 1 } else { 0 }
                | if bits_to_process > 2 && input[start_lane + 2] == T::TRUE { 1 << 2 } else { 0 }
                | if bits_to_process > 3 && input[start_lane + 3] == T::TRUE { 1 << 3 } else { 0 }
                | if bits_to_process > 4 && input[start_lane + 4] == T::TRUE { 1 << 4 } else { 0 }
                | if bits_to_process > 5 && input[start_lane + 5] == T::TRUE { 1 << 5 } else { 0 }
                | if bits_to_process > 6 && input[start_lane + 6] == T::TRUE { 1 << 6 } else { 0 }
                | if bits_to_process > 7 && input[start_lane + 7] == T::TRUE { 1 << 7 } else { 0 };

            assert!(
                bits_to_process < 1
                    || input[start_lane] == T::TRUE
                    || input[start_lane] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 2
                    || input[start_lane + 1] == T::TRUE
                    || input[start_lane + 1] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 3
                    || input[start_lane + 2] == T::TRUE
                    || input[start_lane + 2] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 4
                    || input[start_lane + 3] == T::TRUE
                    || input[start_lane + 3] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 5
                    || input[start_lane + 4] == T::TRUE
                    || input[start_lane + 4] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 6
                    || input[start_lane + 5] == T::TRUE
                    || input[start_lane + 5] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 7
                    || input[start_lane + 6] == T::TRUE
                    || input[start_lane + 6] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
            assert!(
                bits_to_process < 8
                    || input[start_lane + 7] == T::TRUE
                    || input[start_lane + 7] == T::FALSE,
                "Masks values should either be 0 or -1"
            );
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
    use std::intrinsics::simd::*;
    use std::{fmt::Debug, simd::*};

    /// Test that the `simd_bitmask` model is equivalent to the intrinsic for all true and all false
    /// masks with lanes represented using i16.
    #[test]
    fn test_bitmask_i16() {
        check_portable_bitmask::<_, i16, 16, u16>(mask16x16::splat(false));
        check_portable_bitmask::<_, i16, 16, u16>(mask16x16::splat(true));
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
        check_portable_bitmask::<_, i32, 8, u8>(mask32x8::from([
            true, true, false, true, false, false, false, true,
        ]));

        check_portable_bitmask::<_, i32, 4, u8>(mask32x4::from([true, false, false, true]));
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
    fn check_portable_bitmask<T, E, const LANES: usize, M>(mask: Mask<T, LANES>)
    where
        T: std::simd::MaskElement,
        LaneCount<LANES>: SupportedLaneCount,
        E: kani_intrinsic::MaskElement,
        [u8; kani_intrinsic::mask_len(LANES)]: Sized,
        u64: From<M>,
    {
        assert_eq!(
            unsafe { u64::from(kani_intrinsic::simd_bitmask::<_, M, E, LANES>(mask)) },
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

    /// Similar to portable simd_harness.
    #[test]
    fn check_mask_harness() {
        // From array doesn't work either. Manually build [false, true, false, true]
        let mut mask = mask32x4::splat(false);
        mask.set(1, true);
        mask.set(3, true);
        let bitmask = mask.to_bitmask();
        assert_eq!(bitmask, 0b1010);

        let kani_mask = unsafe { u64::from(kani_intrinsic::simd_bitmask::<_, u8, u32, 4>(mask)) };
        assert_eq!(kani_mask, bitmask);
    }
}
