// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfloat-lib

//! This test checks that `kani::float::float_to_int_in_range` works as expected

#[kani::proof]
fn check_float_to_int_in_range() {
    let f: f32 = 5.6;
    let fits_in_u16 = kani::float::float_to_int_in_range::<f32, u16>(f);
    assert!(fits_in_u16);
    let i: u16 = unsafe { f.to_int_unchecked() };
    assert_eq!(i, 5);

    let f: f32 = 145.7;
    let fits_in_i8 = kani::float::float_to_int_in_range::<f32, i8>(f);
    // doesn't fit in `i8` because the value after truncation (`145.0`) is larger than `i8::MAX`
    assert!(!fits_in_i8);

    let f: f64 = 1e6;
    let fits_in_u32 = kani::float::float_to_int_in_range::<f64, u32>(f);
    // fits in `u32` because the value after truncation (`1e6`) is smaller than `u32::MAX`
    assert!(fits_in_u32);
    let i: u32 = unsafe { f.to_int_unchecked() };
    assert_eq!(i, 1_000_000);
}
