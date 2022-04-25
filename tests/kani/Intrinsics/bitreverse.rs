// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we get the expected results for the `bitreverse` intrinsic
// https://doc.rust-lang.org/std/intrinsics/fn.bitreverse.html

const BITS_PER_BYTE: usize = 8;

macro_rules! test_bitreverse_intrinsic {
    ($ty:ty, $check_name:ident, $get_bit_name:ident) => {
        fn $get_bit_name(x: $ty, n: usize) -> bool {
            return x & (1 << n) != 0;
        }

        fn $check_name(a: $ty, b: $ty) {
            let len: usize = (std::mem::size_of::<$ty>() * BITS_PER_BYTE);
            let n: usize = kani::any();
            kani::assume(n < len);
            assert!($get_bit_name(a, n) == $get_bit_name(b, (len - 1) - n));
        }

        let x: $ty = kani::any();
        $check_name(x, x.reverse_bits());
    };
}

#[allow(overflowing_literals)]
#[kani::proof]
fn main() {
    test_bitreverse_intrinsic!(u8, check_reverse_u8, get_bit_at_u8);
    test_bitreverse_intrinsic!(u16, check_reverse_u16, get_bit_at_u16);
    test_bitreverse_intrinsic!(u32, check_reverse_u32, get_bit_at_u32);
    test_bitreverse_intrinsic!(u64, check_reverse_u64, get_bit_at_u64);
    test_bitreverse_intrinsic!(u128, check_reverse_u128, get_bit_at_u128);
    test_bitreverse_intrinsic!(usize, check_reverse_usize, get_bit_at_usize);
    // `reverse_bits` is also defined for signed integer types by casting the
    // number into the corresponding unsigned type and then casting the result
    // into the original signed type. This causes overflows unless we restrict
    // their values considerably, making the signed versions not very
    // interesting to test here.
}
