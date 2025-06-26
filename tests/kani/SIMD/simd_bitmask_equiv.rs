// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(repr_simd, core_intrinsics)]
#![feature(generic_const_exprs)]
#![feature(portable_simd)]

// This test checks the equivalence of Kani's old and new implementations of the
// `simd_bitmask` intrinsic

use std::fmt::Debug;

pub trait MaskElement: PartialEq + Debug {
    const TRUE: Self;
    const FALSE: Self;
}

impl MaskElement for i32 {
    const TRUE: Self = -1;
    const FALSE: Self = 0;
}

/// Calculate the minimum number of lanes to represent a mask
/// Logic similar to `bitmask_len` from `portable_simd`.
/// <https://github.com/rust-lang/portable-simd/blob/490b5cf/crates/core_simd/src/masks/to_bitmask.rs#L75-L79>
const fn mask_len(len: usize) -> usize {
    len.div_ceil(8)
}

fn simd_bitmask_impl_old<T, const LANES: usize>(input: &[T; LANES]) -> [u8; mask_len(LANES)]
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

unsafe fn simd_bitmask<T, U, E, const LANES: usize>(input: T) -> U
where
    [u8; mask_len(LANES)]: Sized,
    E: MaskElement,
{
    let data = &*(&input as *const T as *const [E; LANES]);
    let mask = simd_bitmask_impl_old(data);
    (&mask as *const [u8; mask_len(LANES)] as *const U).read()
}

#[repr(simd)]
#[derive(Clone, Debug)]
struct CustomMask<const LANES: usize>([i32; LANES]);

impl<const LANES: usize> kani::Arbitrary for CustomMask<LANES>
where
    [bool; LANES]: Sized + kani::Arbitrary,
{
    fn any() -> Self {
        CustomMask(kani::any::<[bool; LANES]>().map(|v| if v { i32::FALSE } else { i32::TRUE }))
    }
}

#[kani::proof]
#[kani::solver(kissat)]
fn check_equiv() {
    let mask = kani::any::<CustomMask<8>>();
    unsafe {
        let result1 = simd_bitmask::<_, u8, i32, 8>(mask.clone());
        let result2 = std::intrinsics::simd::simd_bitmask::<_, u8>(mask);
        assert_eq!(result1, result2);
    }
}
