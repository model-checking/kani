// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Test that the automatic harness generation feature filters functions correctly,
// i.e., that it generates harnesses for a function iff:
//   - It is not itself a harness
//   - All of its arguments implement Arbitrary, either trivially or through a user-provided implementation
// The bodies of these functions are purposefully left as simple as possible;
// the point is not to test the generated harnesses themselves,
// but only that we generate the harnesses in the first place.

#![feature(f16)]
#![feature(f128)]

extern crate kani;
use kani::Arbitrary;

#[derive(Arbitrary)]
struct DerivesArbitrary {
    x: u8,
    y: u32,
}

struct ManuallyImplementsArbitrary {
    x: u8,
    y: u32,
}

impl Arbitrary for ManuallyImplementsArbitrary {
    fn any() -> Self {
        Self { x: kani::any(), y: kani::any() }
    }
}

struct DoesntImplementArbitrary {
    x: u8,
    y: u32,
}

mod yes_harness {
    use crate::{DerivesArbitrary, ManuallyImplementsArbitrary};
    use std::marker::{PhantomData, PhantomPinned};
    use std::mem::MaybeUninit;
    use std::num::NonZero;

    // Kani-provided Arbitrary implementations
    fn f_u8(x: u8) -> u8 {
        x
    }
    fn f_u16(x: u16) -> u16 {
        x
    }
    fn f_u32(x: u32) -> u32 {
        x
    }
    fn f_u64(x: u64) -> u64 {
        x
    }
    fn f_u128(x: u128) -> u128 {
        x
    }
    fn f_usize(x: usize) -> usize {
        x
    }
    fn f_i8(x: i8) -> i8 {
        x
    }
    fn f_i16(x: i16) -> i16 {
        x
    }
    fn f_i32(x: i32) -> i32 {
        x
    }
    fn f_i64(x: i64) -> i64 {
        x
    }
    fn f_i128(x: i128) -> i128 {
        x
    }
    fn f_isize(x: isize) -> isize {
        x
    }
    fn f_bool(x: bool) -> bool {
        x
    }
    fn f_char(x: char) -> char {
        x
    }
    fn f_f32(x: f32) -> f32 {
        x
    }
    fn f_f64(x: f64) -> f64 {
        x
    }
    fn f_f16(x: f16) -> f16 {
        x
    }
    fn f_f128(x: f128) -> f128 {
        x
    }
    fn f_nonzero_u8(x: NonZero<u8>) -> NonZero<u8> {
        x
    }
    fn f_nonzero_u16(x: NonZero<u16>) -> NonZero<u16> {
        x
    }
    fn f_nonzero_u32(x: NonZero<u32>) -> NonZero<u32> {
        x
    }
    fn f_nonzero_u64(x: NonZero<u64>) -> NonZero<u64> {
        x
    }
    fn f_nonzero_u128(x: NonZero<u128>) -> NonZero<u128> {
        x
    }
    fn f_nonzero_usize(x: NonZero<usize>) -> NonZero<usize> {
        x
    }
    fn f_nonzero_i8(x: NonZero<i8>) -> NonZero<i8> {
        x
    }
    fn f_nonzero_i16(x: NonZero<i16>) -> NonZero<i16> {
        x
    }
    fn f_nonzero_i32(x: NonZero<i32>) -> NonZero<i32> {
        x
    }
    fn f_nonzero_i64(x: NonZero<i64>) -> NonZero<i64> {
        x
    }
    fn f_nonzero_i128(x: NonZero<i128>) -> NonZero<i128> {
        x
    }
    fn f_nonzero_isize(x: NonZero<isize>) -> NonZero<isize> {
        x
    }
    fn f_array(x: [u8; 4]) -> [u8; 4] {
        x
    }
    fn f_option(x: Option<u8>) -> Option<u8> {
        x
    }
    fn f_result(x: Result<u8, u16>) -> Result<u8, u16> {
        x
    }
    fn f_maybe_uninit(x: MaybeUninit<u8>) -> MaybeUninit<u8> {
        x
    }
    fn f_tuple(x: (u8, u16, u32)) -> (u8, u16, u32) {
        x
    }

    // The return type doesn't implement Arbitrary, but that shouldn't matter
    fn f_unsupported_return_type(x: u8) -> Vec<u8> {
        vec![x]
    }

    // Multiple arguments of different types, all of which implement Arbitrary
    fn f_multiple_args(x: u8, y: u16, z: u32) -> (u8, u16, u32) {
        (x, y, z)
    }

    // User-defined types that implement Arbitrary
    fn f_derives_arbitrary(x: DerivesArbitrary) -> DerivesArbitrary {
        x
    }
    fn f_manually_implements_arbitrary(
        x: ManuallyImplementsArbitrary,
    ) -> ManuallyImplementsArbitrary {
        x
    }

    fn f_phantom_data(x: PhantomData<u8>) -> PhantomData<u8> {
        x
    }

    fn f_phantom_pinned(x: PhantomPinned) -> PhantomPinned {
        x
    }

    fn empty_body(_x: u8, _y: u16) {}
}

mod no_harness {
    use crate::{DerivesArbitrary, DoesntImplementArbitrary};
    fn unsupported_generic<T>(x: u32, _y: T) -> u32 {
        x
    }
    fn unsupported_ref(x: u32, _y: &i32) -> u32 {
        x
    }
    fn unsupported_const_pointer(x: u32, _y: *const i32) -> u32 {
        x
    }
    fn unsupported_mut_pointer(x: u32, _y: *mut i32) -> u32 {
        x
    }
    fn unsupported_vec(x: u32, _y: Vec<u8>) -> u32 {
        x
    }
    fn unsupported_slice(x: u32, _y: &[u8]) -> u32 {
        x
    }
    fn doesnt_implement_arbitrary(
        x: DoesntImplementArbitrary,
        _y: DerivesArbitrary,
    ) -> DoesntImplementArbitrary {
        x
    }
    // Test that we correctly render the name of the argument "_" in the table of skipped functions
    // (this argument will have no var_debug_info from StableMIR, unlike arguments with names)
    fn unsupported_no_arg_name(_: &()) {}
}
