// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `cttz` and `cttz_nonzero` are supported and return the expected
// results

#![feature(core_intrinsics)]
use std::intrinsics::{cttz, cttz_nonzero};

// Define a function for counting like `cttz` and assert that their results are
// the same for any value
macro_rules! test_cttz {
    ( $fn_name:ident, $ty:ty ) => {
        fn $fn_name(x: $ty) -> u32 {
            let mut count = 0;
            let num_bits = <$ty>::BITS;
            for i in 0..num_bits {
                // Get value at index `i`
                let bitmask = 1 << i;
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
        assert!($fn_name(var) == cttz(var));
        // Check that the stable version returns the same value
        assert!(cttz(var) as u32 == var.trailing_zeros());
    };
}

// Assert that the results of `cttz` and `cttz_nonzero` are the same if we
// exclude zero
macro_rules! test_cttz_nonzero {
    ($ty:ty) => {
        let var_nonzero: $ty = kani::any();
        kani::assume(var_nonzero != 0);
        unsafe {
            assert!(cttz(var_nonzero) == cttz_nonzero(var_nonzero));
        }
    };
}

#[kani::proof]
fn main() {
    test_cttz!(my_cttz_u8, u8);
    test_cttz!(my_cttz_u16, u16);
    test_cttz!(my_cttz_u32, u32);
    test_cttz!(my_cttz_u64, u64);
    test_cttz!(my_cttz_u128, u128);
    test_cttz!(my_cttz_usize, usize);
    test_cttz_nonzero!(u8);
    test_cttz_nonzero!(u16);
    test_cttz_nonzero!(u32);
    test_cttz_nonzero!(u64);
    test_cttz_nonzero!(u128);
    test_cttz_nonzero!(usize);
    // These intrinsics are also defined for signed integer types by casting the
    // number into the corresponding unsigned type and then casting the result
    // into the original signed type. This causes overflows unless we restrict
    // their values, making the signed versions not very interesting to test
    // here.
}
