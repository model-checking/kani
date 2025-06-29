// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani uses its hook for align_offset.

#[kani::proof]
fn align_offset() {
    let x = 10;
    let ptr = &x as *const i32;
    let alignment = ptr.align_offset(1);
    assert_eq!(alignment, usize::MAX);
}
