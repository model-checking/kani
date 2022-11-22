// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test ensures we detect overflows in SIMD arithmetic operations
#![feature(repr_simd, platform_intrinsics)]

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct i8x2(i8, i8);

extern "platform-intrinsic" {
    fn simd_add<T>(x: T, y: T) -> T;
    fn simd_sub<T>(x: T, y: T) -> T;
    fn simd_mul<T>(x: T, y: T) -> T;
}

#[kani::proof]
fn main() {
    let val_any = kani::any();
    let simd_any = i8x2(val_any, val_any);

    unsafe {
        let _ = simd_add(simd_any, simd_any);
        let _ = simd_sub(simd_any, simd_any);
        let _ = simd_mul(simd_any, simd_any);
    }
}
