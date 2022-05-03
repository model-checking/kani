// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `bswap` returns the expected results.
// https://doc.rust-lang.org/std/intrinsics/fn.bswap.html

// `bswap` reverses the bytes in an integer type `T`, for example:
//
// ```
// let n = 0x12345678u32;
// let m = std::intrinsics::bswap(n);

// assert_eq!(m, 0x78563412);
// ```

#![feature(core_intrinsics)]

const BITS_PER_BYTE: usize = 8;

macro_rules! test_bswap_intrinsic {
    ($ty:ty, $check_name:ident, $get_byte_name:ident) => {
        // Gets the i-th byte in `x`
        fn $get_byte_name(x: $ty, i: usize) -> $ty {
                let mask = 0xFF as $ty << i * BITS_PER_BYTE;
                let masked_res = x & mask;
                let bytes_res = masked_res >> i * BITS_PER_BYTE;
                bytes_res
        }

        // Checks that the order of bytes in the original integer is reversed in
        // the swapped integer
        fn $check_name(a: $ty, b: $ty) {
            let bytes = std::mem::size_of::<$ty>();
            let i: usize = kani::any();
            kani::assume(i < bytes);
            let a_byte = $get_byte_name(a, i);
            let b_byte = $get_byte_name(b, bytes - i - 1);
            assert!(a_byte == b_byte);
        }

        let x: $ty = kani::any();
        $check_name(x, std::intrinsics::bswap(x));
    }
}

#[kani::proof]
fn test_bswap_u8() {
    test_bswap_intrinsic!(u8, check_bswap_u8, get_byte_at_u8);
}

#[kani::proof]
fn test_bswap_u16() {
    test_bswap_intrinsic!(u16, check_bswap_u16, get_byte_at_u16);
}

#[kani::proof]
fn test_bswap_u32() {
    test_bswap_intrinsic!(u32, check_bswap_u32, get_byte_at_u32);
}

#[kani::proof]
fn test_bswap_u64() {
    test_bswap_intrinsic!(u64, check_bswap_u64, get_byte_at_u64);
}

#[kani::proof]
fn test_bswap_u128() {
    test_bswap_intrinsic!(u128, check_bswap_u128, get_byte_at_u128);
}

// `bswap` also works with signed integer types, but this causes overflows
// unless we restrict their values considerably (due to how bytes are
// extracted), making the signed versions not very interesting to test here.
// https://github.com/model-checking/kani/issues/934
