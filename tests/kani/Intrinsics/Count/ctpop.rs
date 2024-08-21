// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `ctpop` is supported and returns the expected results
// (the number of bits equal to one in a value)
#![feature(core_intrinsics)]
use std::intrinsics::ctpop;

// Define a function for counting like `ctpop` and assert that their results are
// the same for any value
macro_rules! test_ctpop {
    ( $fn_name:ident, $ty:ty ) => {
        fn $fn_name(x: $ty) -> u32 {
            let mut count = 0;
            let num_bits = <$ty>::BITS;
            for i in 0..num_bits {
                // Get value at index `i`
                let bitmask = 1 << i;
                let bit = x & bitmask;
                if bit != 0 {
                    count += 1;
                }
            }
            count
        }
        let var: $ty = kani::any();
        // Check that the result is correct
        assert!($fn_name(var) == ctpop(var));

        // Check that the stable version returns the same value
        assert!(ctpop(var) as u32 == var.count_ones());
    };
}

#[kani::proof]
fn test_ctpop_u8() {
    test_ctpop!(my_ctpop_u8, u8);
}

#[kani::proof]
fn test_ctpop_u16() {
    test_ctpop!(my_ctpop_u16, u16);
}

#[kani::proof]
fn test_ctpop_u32() {
    test_ctpop!(my_ctpop_u32, u32);
}

#[kani::proof]
fn test_ctpop_u64() {
    test_ctpop!(my_ctpop_u64, u64);
}

// We do not run the test for u128 because it takes too long
fn test_ctpop_u128() {
    test_ctpop!(my_ctpop_u128, u128);
}

#[kani::proof]
fn test_ctpop_usize() {
    test_ctpop!(my_ctpop_usize, usize);
}

// `ctpop` also works with signed integer types, but this causes overflows
// unless we restrict their values considerably (due to the conversions in
// `count_ones`), making the signed versions not very interesting to test here.
// https://github.com/model-checking/kani/issues/934
