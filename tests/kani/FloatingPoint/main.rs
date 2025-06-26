// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(f16)]
#![feature(f128)]
#![feature(repr_simd)]

macro_rules! test_floats {
    ($ty:ty) => {
        let a: $ty = kani::any();
        let b = a / 2.0;

        if a < 0.0 {
            assert!(a <= b);
        } else if a >= 0.0 {
            assert!(a >= b);
        }

        let c = b * 2.0;
        // general/infinity            Close but not exact                    NAN
        assert!(a == c || a - c < 0.00000001 || c - a < 0.00000001 || c * 0.0 != 0.0);

        let d: $ty = 0.0;
        assert!(d + 1.0 > 0.0);
        assert!(d - 1.0 < 0.0);
    };
}

#[kani::proof]
fn main() {
    assert!(1.1 == 1.1 * 1.0);
    assert!(1.1 != 1.11 / 1.0);

    test_floats!(f16);
    test_floats!(f32);
    test_floats!(f64);
    test_floats!(f128);
}

// Test that we can codegen floats when we hit them in codegen_float_type,
// c.f. https://github.com/model-checking/kani/issues/3069#issuecomment-2730501056
#[repr(simd)]
struct f16x16([f16; 16]);

fn make_float_array() -> f16x16 {
    f16x16([1.0; 16])
}

#[kani::proof]
fn make_float_array_harness() {
    let _ = make_float_array();
}
