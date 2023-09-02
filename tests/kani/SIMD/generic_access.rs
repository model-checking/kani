// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure we can use a generic function to access field of `repr_simd`.
#![feature(portable_simd, repr_simd)]

use std::simd::SimdElement;

mod array_based {
    use super::*;

    #[repr(simd)]
    struct CustomSimd<T: SimdElement, const LANES: usize>([T; LANES]);

    fn check_fields<T: SimdElement + PartialEq, const LANES: usize>(
        simd: CustomSimd<T, LANES>,
        expected: [T; LANES],
    ) {
        assert_eq!(simd.0, expected);
    }

    #[kani::proof]
    fn check_field_access() {
        let data: [u8; 16] = kani::any();
        let vec = CustomSimd(data.clone());
        check_fields(vec, data);
    }
}

mod fields_based {
    use super::*;

    #[repr(simd)]
    struct CustomSimd<T: SimdElement>(T, T);

    fn check_fields<T: SimdElement + PartialEq, const LANES: usize>(
        simd: CustomSimd<T>,
        expected: [T; LANES],
    ) {
        assert_eq!(simd.0, expected[0]);
        assert_eq!(simd.1, expected[1])
    }

    #[kani::proof]
    fn check_field_access() {
        let data: [u8; 16] = kani::any();
        let vec = CustomSimd(data[0], data[1]);
        check_fields(vec, data);
    }
}
