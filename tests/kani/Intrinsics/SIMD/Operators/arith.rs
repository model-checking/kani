// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test doesn't work because support for SIMD intrinsics isn't available
//! at the moment in Kani. Support to be added in
//! <https://github.com/model-checking/kani/issues/1148>
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct i8x2(i8, i8);

extern "platform-intrinsic" {
    fn simd_add<T>(x: T, y: T) -> T;
    fn simd_sub<T>(x: T, y: T) -> T;
    fn simd_mul<T>(x: T, y: T) -> T;
}

macro_rules! verify_no_overflow {
    ($cf: ident, $uf: ident) => {{
        let a: i8 = kani::any();
        let b: i8 = kani::any();
        let checked = a.$cf(b);
        kani::assume(checked.is_some());
        let simd_a = i8x2(a, a);
        let simd_b = i8x2(b, b);
        let unchecked: i8x2 = unsafe { $uf(simd_a, simd_b) };
        assert!(checked.unwrap() == unchecked.0);
        assert!(checked.unwrap() == unchecked.1);
    }};
}

#[kani::proof]
fn test_simd_ops() {
    verify_no_overflow!(checked_add, simd_add);
    verify_no_overflow!(checked_sub, simd_sub);
    verify_no_overflow!(checked_mul, simd_mul);
}
