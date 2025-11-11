// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Fix for array-based SIMD projection mismatch issue #2264
//!
//! This test verifies that projection mismatches with array-based SIMD types
//! no longer occur after the fix in the check_expr_typ_mismatch function.

#![feature(repr_simd)]

#[derive(Copy)]
#[repr(simd)]
struct ArraySimd<T>([T; 4]);

impl<T: Copy> Clone for ArraySimd<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[kani::proof]
fn test_array_simd_transmute() {
    let v = ArraySimd::<u32>([1, 2, 3, 4]);

    // These operations previously caused projection mismatch errors
    // but should now work correctly
    unsafe {
        let _array: [u32; 4] = std::mem::transmute(v);
        let _same_size_different_type: ArraySimd<i32> = std::mem::transmute(v);
        let _same_size_different_scalar: ArraySimd<f32> = std::mem::transmute(v);
    }
}

#[kani::proof]
fn test_different_array_sizes() {
    let v2 = ArraySimd::<u16>([1, 2, 3, 4]);

    // Test conversions to array types
    unsafe {
        let _arr2: [u16; 4] = std::mem::transmute(v2);

        // Test same total bit size conversion (u16[4] = 64 bits = u32[2] = 64 bits)
        let v2_as_u32: [u32; 2] = std::mem::transmute(v2);
        assert_eq!(v2_as_u32.len(), 2);
    }
}

#[kani::proof]
fn test_field_access_equivalent() {
    let v = ArraySimd::<i32>([10, 20, 30, 40]);

    // Test that we can access the underlying data
    unsafe {
        let arr: [i32; 4] = std::mem::transmute(v);
        assert_eq!(arr[0], 10);
        assert_eq!(arr[1], 20);
        assert_eq!(arr[2], 30);
        assert_eq!(arr[3], 40);
    }
}
