// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that we support `simd_bitmask` intrinsic.
//!
//! This is done by initializing vectors with the contents of 2-member tuples
//! with symbolic values. The result of using each of the intrinsics is compared
//! against the result of using the associated bitwise operator on the tuples.
#![feature(repr_simd, core_intrinsics)]
#![feature(generic_const_exprs)]
#![feature(portable_simd)]
use std::fmt::Debug;
use std::intrinsics::simd::simd_bitmask;

#[repr(simd)]
#[derive(Clone, Debug)]
struct CustomMask<const LANES: usize>([i32; LANES]);

impl<const LANES: usize> CustomMask<LANES> {
    fn as_array(&self) -> [i32; LANES] {
        unsafe { *(&self.clone() as *const Self as *const [i32; LANES]) }
    }
}

impl<const LANES: usize> kani::Arbitrary for CustomMask<LANES>
where
    [bool; LANES]: Sized + kani::Arbitrary,
{
    fn any() -> Self {
        CustomMask(kani::any::<[bool; LANES]>().map(|v| if v { 0i32 } else { -1 }))
    }
}

#[kani::proof]
fn check_u8() {
    let (true_lane, false_lane) = (-1, 0);

    // This should be the mask 0b1101 for little endian machines.
    let input = CustomMask([true_lane, false_lane, true_lane, true_lane]);
    let mask = unsafe { simd_bitmask::<_, u8>(input) };
    assert_eq!(mask, 0b1101);

    let input = CustomMask([true_lane; 25]);
    let mask = unsafe { simd_bitmask::<_, u32>(input) };
    assert_eq!(mask, 0b1111111111111111111111111);
}

#[kani::proof]
fn check_unsigned_bitmask() {
    let mask = kani::any::<CustomMask<8>>();
    let bitmask = unsafe { simd_bitmask::<_, u8>(mask.clone()) };
    assert_eq!(bitmask.count_ones() as usize, mask.as_array().iter().filter(|e| **e == -1).count());
    assert_eq!(bitmask.count_zeros() as usize, mask.as_array().iter().filter(|e| **e == 0).count());
}
