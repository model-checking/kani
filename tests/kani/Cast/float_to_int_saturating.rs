// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression test for float-to-int saturating cast (GitHub issue #4536).
//! Since Rust 1.45, `as` performs saturating casts from float to int.

// Test case from issue #4536
#[kani::proof]
fn check_issue_4536_f32_to_u8() {
    let x: f32 = 300.0;
    let y: u8 = x as u8;
    assert!(y == 255); // Should saturate to u8::MAX
}

// f32 saturating casts
#[kani::proof]
fn check_f32_to_u8_above_max() {
    let y: u8 = 300.0f32 as u8;
    assert!(y == u8::MAX);
}

#[kani::proof]
fn check_f32_to_u8_below_min() {
    let y: u8 = (-10.0f32) as u8;
    assert!(y == 0);
}

#[kani::proof]
fn check_f32_to_u8_nan() {
    let y: u8 = f32::NAN as u8;
    assert!(y == 0);
}

#[kani::proof]
fn check_f32_to_u8_infinity() {
    let y: u8 = f32::INFINITY as u8;
    assert!(y == u8::MAX);
}

#[kani::proof]
fn check_f32_to_i8_above_max() {
    let y: i8 = 200.0f32 as i8;
    assert!(y == i8::MAX);
}

#[kani::proof]
fn check_f32_to_i8_below_min() {
    let y: i8 = (-200.0f32) as i8;
    assert!(y == i8::MIN);
}

// f64 saturating casts
#[kani::proof]
fn check_f64_to_u8_above_max() {
    let y: u8 = 300.0f64 as u8;
    assert!(y == u8::MAX);
}

#[kani::proof]
fn check_f64_to_i8_below_min() {
    let y: i8 = (-200.0f64) as i8;
    assert!(y == i8::MIN);
}

// In-range casts should truncate toward zero
#[kani::proof]
fn check_f32_truncation() {
    let y: i8 = (-99.9f32) as i8;
    assert!(y == -99);
}
