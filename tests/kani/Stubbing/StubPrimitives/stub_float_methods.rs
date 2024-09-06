// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub float functions.
#![feature(f128)]
#![feature(f16)]

/// Generate stub and harness for floor method on floats.
macro_rules! stub_floor {
    ($ty:ty, $harness:ident, $stub:ident) => {
        // Stub that always returns 0.
        pub fn $stub(_: $ty) -> $ty {
            0.0
        }

        // Harness
        #[kani::proof]
        #[kani::stub($ty::floor, $stub)]
        pub fn $harness() {
            let input = kani::any();
            let floor = <$ty>::floor(input);
            assert_eq!(floor, 0.0);
        }
    };
}

stub_floor!(f16, f16_floor, stub_f16_floor);
stub_floor!(f32, f32_floor, stub_f32_floor);
stub_floor!(f64, f64_floor, stub_f64_floor);
stub_floor!(f128, f128_floor, stub_f128_floor);
