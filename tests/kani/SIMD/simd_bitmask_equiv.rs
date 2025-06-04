// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(generic_const_exprs)]

// This test checks the equivalence of Kani's old and new implementations of the
// `simd_bitmask` intrinsic

use std::fmt::Debug;

pub trait MaskElement: PartialEq + Debug {
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
pub const fn mask_len(len: usize) -> usize {
    len.div_ceil(8)
}

pub fn simd_bitmask_impl1<T, const LANES: usize>(input: &[T; LANES]) -> [u8; mask_len(LANES)]
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

pub fn simd_bitmask_impl2<T, const LANES: usize>(input: &[T; LANES]) -> [u8; mask_len(LANES)]
where
    T: MaskElement,
{
    let mut mask_array = [0; mask_len(LANES)];
    let mask_len = mask_array.len();

    // Process 8 lanes at a time when possible
    for byte in 0..mask_len {
        // Calculate the starting lane for this byte
        let start_lane = byte * 8;
        // Calculate how many bits to process (handle the last byte which might be partial)
        let bits_to_process = (LANES - start_lane).min(8);

        // disable formatting for this code block to make it easier to edit/read
        #[rustfmt::skip]
        let byte_mask =
            if bits_to_process > 0 && input[start_lane + 0] == T::TRUE { 1 << 0 } else { 0 } |
            if bits_to_process > 1 && input[start_lane + 1] == T::TRUE { 1 << 1 } else { 0 } |
            if bits_to_process > 2 && input[start_lane + 2] == T::TRUE { 1 << 2 } else { 0 } |
            if bits_to_process > 3 && input[start_lane + 3] == T::TRUE { 1 << 3 } else { 0 } |
            if bits_to_process > 4 && input[start_lane + 4] == T::TRUE { 1 << 4 } else { 0 } |
            if bits_to_process > 5 && input[start_lane + 5] == T::TRUE { 1 << 5 } else { 0 } |
            if bits_to_process > 6 && input[start_lane + 6] == T::TRUE { 1 << 6 } else { 0 } |
            if bits_to_process > 7 && input[start_lane + 7] == T::TRUE { 1 << 7 } else { 0 };

        assert!(
            bits_to_process < 1
                || input[start_lane + 0] == T::TRUE
                || input[start_lane + 0] == T::FALSE,
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

        mask_array[byte] = byte_mask;
    }

    mask_array
}

#[kani::proof]
#[kani::solver(kissat)]
fn check_equiv() {
    let input: [bool; 17] = kani::any();
    let input: [i8; 17] = input.map(|x| if x { i8::TRUE } else { i8::FALSE });
    let result1 = simd_bitmask_impl1(&input);
    let result2 = simd_bitmask_impl2(&input);
    assert_eq!(result1, result2);
}
