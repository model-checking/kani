// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z unstable-options --prove-safety-only
//! Test that --prove-safety-only works

#[kani::proof]
fn div0() -> i32 {
    let x: i32 = kani::any();
    let y: i32 = kani::any();
    x / y
}

#[kani::proof]
fn assert_hides_ub() {
    let arr: [u8; 5] = kani::any();
    let mut bytes = kani::slice::any_slice_of_array(&arr);
    let slice_offset = unsafe { bytes.as_ptr().offset_from(&arr as *const u8) };
    let offset: usize = kani::any();
    assert!(offset <= 4 && (slice_offset as usize) + offset <= 4);
    let _ = unsafe { *bytes.as_ptr().add(offset) };
}
