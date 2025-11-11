// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test for projection mismatch fix with array-based SIMD types.
//! This addresses the issue described in https://github.com/model-checking/kani/issues/2264
//! where array-based SIMD types would cause projection mismatches during type conversions.

#![feature(repr_simd)]

#[derive(Copy)]
#[repr(simd)]
struct V<T>([T; 2]);

impl<T: Copy> Clone for V<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy)]
#[repr(simd)]
struct VF32([f32; 2]);

impl Clone for VF32 {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Copy)]
#[repr(simd)]
struct VU32([u32; 2]);

impl Clone for VU32 {
    fn clone(&self) -> Self {
        *self
    }
}

// Test transmute between SIMD types with same representation size
// This should work with the projection fix for array-based SIMD types
fn test_simd_transmute_same_size() {
    let v_f32 = VF32([1.0f32, 2.0f32]);
    let v_u32: VU32 = unsafe { std::mem::transmute(v_f32) };

    // Verify the transmute worked by checking bit patterns
    let f32_bits = 1.0f32.to_bits();
    let u32_val = unsafe { std::mem::transmute::<VU32, [u32; 2]>(v_u32) };
    assert_eq!(u32_val[0], f32_bits);
}

// Test field access on array-based SIMD (this was part of the original issue)
fn test_simd_field_access() {
    let v = V::<u32>([u32::MIN, u32::MAX]);

    // This should work without projection mismatch errors
    unsafe {
        let arr: [u32; 2] = std::mem::transmute(v);
        assert_eq!(arr[0], u32::MIN);
        assert_eq!(arr[1], u32::MAX);
    }
}

#[kani::proof]
fn verify_simd_transmute_same_size() {
    test_simd_transmute_same_size();
}

#[kani::proof]
fn verify_simd_field_access() {
    test_simd_field_access();
}

#[kani::proof]
fn verify_simd_clone() {
    let v = V::<i32>([42, -42]);
    let v2 = v.clone();

    unsafe {
        let arr1: [i32; 2] = std::mem::transmute(v);
        let arr2: [i32; 2] = std::mem::transmute(v2);
        assert_eq!(arr1[0], arr2[0]);
        assert_eq!(arr1[1], arr2[1]);
    }
}
