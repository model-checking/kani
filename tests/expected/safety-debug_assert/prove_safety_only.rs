// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z unstable-options --prove-safety-only
//! Test that --prove-safety-only turns debug_assert into a no-op

#[kani::proof]
fn debug_assert_does_not_hide_ub() {
    let arr: [u8; 5] = kani::any();
    let bytes = kani::slice::any_slice_of_array(&arr);
    let slice_offset = unsafe { bytes.as_ptr().offset_from(&arr as *const u8) };
    let offset: usize = kani::any();
    debug_assert!(offset <= 4 && (slice_offset as usize) + offset <= 4);
    let _ = unsafe { *bytes.as_ptr().add(offset) };
}
