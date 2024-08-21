// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `ctlz` and `ctlz_nonzero` are supported and return the expected
// results

#![feature(core_intrinsics)]
use std::intrinsics::{ctlz, ctlz_nonzero};

// Define a function for counting like `ctlz` and assert that their results are
// the same for any value
macro_rules! test_ctlz {
    ( $fn_name:ident, $ty:ty ) => {
        fn $fn_name(x: $ty) -> u32 {
            let mut count = 0;
            let num_bits = <$ty>::BITS;
            for i in 0..num_bits {
                // Get value at index `i`
                let bitmask = 1 << (num_bits - i - 1);
                let bit = x & bitmask;
                if bit == 0 {
                    count += 1;
                } else {
                    break;
                }
            }
            count
        }
        let var: $ty = kani::any();
        // Check that the result is correct
        assert!($fn_name(var) == ctlz(var));
        // Check that the stable version returns the same value
        assert!(ctlz(var) as u32 == var.leading_zeros());
    };
}

// Assert that the results of `ctlz` and `ctlz_nonzero` are the same if we
// exclude zero
macro_rules! test_ctlz_nonzero {
    ($ty:ty) => {
        let var_nonzero: $ty = kani::any();
        kani::assume(var_nonzero != 0);
        unsafe {
            assert!(ctlz(var_nonzero) == ctlz_nonzero(var_nonzero));
        }
    };
}

#[kani::proof]
fn main() {
    test_ctlz!(my_ctlz_u8, u8);
    test_ctlz!(my_ctlz_u16, u16);
    test_ctlz!(my_ctlz_u32, u32);
    test_ctlz!(my_ctlz_u64, u64);
    test_ctlz!(my_ctlz_u128, u128);
    test_ctlz!(my_ctlz_usize, usize);
    test_ctlz_nonzero!(u8);
    test_ctlz_nonzero!(u16);
    test_ctlz_nonzero!(u32);
    test_ctlz_nonzero!(u64);
    test_ctlz_nonzero!(u128);
    test_ctlz_nonzero!(usize);
    // These intrinsics are also defined for signed integer types by casting the
    // number into the corresponding unsigned type and then casting the result
    // into the original signed type. This causes overflows unless we restrict
    // their values, making the signed versions not very interesting to test
    // here.
}
