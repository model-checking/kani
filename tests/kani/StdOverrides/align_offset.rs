// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test makes sure Kani uses its hook for align_offset.

#[kani::proof]
fn align_offset() {
    let x = [10, 42];
    // The hook always returns `usize::MAX`, which `align_offset` explicitly permits: "It is
    // permissible for the implementation to always return usize::MAX." Rust's own implementation
    // returns 0 for both of these pointers when the alignment is 1, so asserting `usize::MAX` is
    // what shows the hook is being used rather than the real implementation.
    let base_ptr = &x[0] as *const i32;
    assert_eq!(base_ptr.align_offset(1), usize::MAX);
    let offset_ptr = &x[1] as *const i32;
    assert_eq!(offset_ptr.align_offset(1), usize::MAX);
}
