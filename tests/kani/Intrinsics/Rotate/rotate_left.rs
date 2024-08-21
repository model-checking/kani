// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `rotate_left` is supported and returns the expected result.
#![feature(core_intrinsics)]
use std::intrinsics::rotate_left;

macro_rules! test_rotate_left {
    ( $fn_name:ident, $ty:ty ) => {
        fn $fn_name(x: $ty, rot_x: $ty, n: u32) {
            let i: u32 = kani::any();
            kani::assume(i < <$ty>::BITS);
            // Get value at index `i`
            let bitmask = 1 << i;
            let bit = (x & bitmask) != 0;
            // Get value at rotated index `rot_i`
            let rot_i = (i + (n as u32)) % <$ty>::BITS;
            let rot_bitmask = 1 << rot_i;
            let rot_bit = (rot_x & rot_bitmask) != 0;
            // Assert that both bit values are the same
            assert!(bit == rot_bit);
        }
        let x: $ty = kani::any();
        let n: u32 = kani::any();
        // Limit `n` to `u8::MAX` to avoid overflows
        kani::assume(n <= u8::MAX as u32);
        let y: $ty = rotate_left(x, n);
        // Check that the rotation is correct
        $fn_name(x, y, n);
        // Check that the stable version returns the same value
        assert!(y == x.rotate_left(n));
    };
}

#[kani::proof]
fn main() {
    test_rotate_left!(check_rol_u8, u8);
    test_rotate_left!(check_rol_u16, u16);
    test_rotate_left!(check_rol_u32, u32);
    test_rotate_left!(check_rol_u64, u64);
    test_rotate_left!(check_rol_u128, u128);
    test_rotate_left!(check_rol_usize, usize);
    // `rotate_left` is also defined for signed integer types by casting the
    // number into the corresponding unsigned type and then casting the result
    // into the original signed type. This causes overflows unless we restrict
    // their values considerably, making the signed versions not very
    // interesting to test here.
}
