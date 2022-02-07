// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `bitreverse` intrinsic
// https://doc.rust-lang.org/std/intrinsics/fn.bitreverse.html

// Note: Support for `__builtin_bitreverse` in CBMC is being added in
// https://github.com/diffblue/cbmc/pull/6581
// Tracking issue: https://github.com/model-checking/kani/issues/778

const BITS_PER_BYTE: usize = 8;

macro_rules! test_bitreverse_intrinsic {
    ($ty:ty, $check_name:ident, $get_bit_name:ident) => {
        fn $get_bit_name(x: $ty, n: usize) -> bool {
            return x & (1 << n) != 0;
        }

        fn $check_name(a: $ty, b: $ty) -> bool {
            let len: usize = (std::mem::size_of::<$ty>() * BITS_PER_BYTE) - 1;
            for n in 0..len {
                if $get_bit_name(a, n) != $get_bit_name(b, len - n) {
                    return false;
                }
            }
            return true;
        }

        let x: $ty = kani::any();
        let res = $check_name(x, x.reverse_bits());
        assert!(res);
    };
}

#[allow(overflowing_literals)]
fn main() {
    test_bitreverse_intrinsic!(u8, check_reverse_u8, get_bit_at_u8);
    test_bitreverse_intrinsic!(u16, check_reverse_u16, get_bit_at_u16);
    test_bitreverse_intrinsic!(u32, check_reverse_u32, get_bit_at_u32);
    test_bitreverse_intrinsic!(u64, check_reverse_u64, get_bit_at_u64);
    test_bitreverse_intrinsic!(usize, check_reverse_usize, get_bit_at_usize);
    test_bitreverse_intrinsic!(i8, check_reverse_i8, get_bit_at_i8);
    test_bitreverse_intrinsic!(i16, check_reverse_i16, get_bit_at_i16);
    test_bitreverse_intrinsic!(i32, check_reverse_i32, get_bit_at_i32);
    test_bitreverse_intrinsic!(i64, check_reverse_i64, get_bit_at_i64);
    test_bitreverse_intrinsic!(isize, check_reverse_isize, get_bit_at_isize);
}
