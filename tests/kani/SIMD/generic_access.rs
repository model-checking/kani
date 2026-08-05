// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Ensure we can use a generic function to access field of `repr_simd`.
#![feature(portable_simd, repr_simd)]

use std::simd::SimdElement;

mod array_based {
    use super::*;

    #[repr(simd)]
    #[derive(Copy)]
    struct CustomSimd<T: SimdElement, const LANES: usize>([T; LANES]);

    impl<T: SimdElement, const LANES: usize> Clone for CustomSimd<T, LANES> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T: SimdElement, const LANES: usize> CustomSimd<T, LANES> {
        fn as_array(&self) -> &[T; LANES] {
            let p: *const Self = self;
            unsafe { &*p.cast::<[T; LANES]>() }
        }

        fn into_array(self) -> [T; LANES]
        where
            T: Copy,
        {
            *self.as_array()
        }
    }

    fn check_fields<T: SimdElement + PartialEq, const LANES: usize>(
        simd: CustomSimd<T, LANES>,
        expected: [T; LANES],
    ) {
        assert_eq!(simd.into_array(), expected);
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
    #[derive(Copy)]
    struct CustomSimd<T: SimdElement>([T; 2]);

    impl<T: SimdElement> Clone for CustomSimd<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T: SimdElement> CustomSimd<T> {
        fn as_array(&self) -> &[T; 2] {
            let p: *const Self = self;
            unsafe { &*p.cast::<[T; 2]>() }
        }

        fn into_array(self) -> [T; 2]
        where
            T: Copy,
        {
            *self.as_array()
        }
    }

    fn check_fields<T: SimdElement + PartialEq, const LANES: usize>(
        simd: CustomSimd<T>,
        expected: [T; LANES],
    ) {
        assert_eq!(simd.into_array()[0], expected[0]);
        assert_eq!(simd.into_array()[1], expected[1])
    }

    #[kani::proof]
    fn check_field_access() {
        let data: [u8; 16] = kani::any();
        let vec = CustomSimd([data[0], data[1]]);
        check_fields(vec, data);
    }
}
