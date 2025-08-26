// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani uses its hook for align_offset.

#[kani::proof]
fn align_offset() {
    let x = [10, 42];
    let base_ptr = &x[0] as *const i32;
    let base_alignment = base_ptr.align_offset(1);
    assert_eq!(base_alignment, 0);
    let offset_ptr = &x[1] as *const i32;
    let offset_alignment = offset_ptr.align_offset(1);
    assert_eq!(offset_alignment, usize::MAX);
}
