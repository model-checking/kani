// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani offset operations correctly detect out-of-bound access.

/// Offset from a pointer that is out of bounds should be invalid even if it goes back to the
/// original allocation.
#[kani::proof]
fn check_add_to_oob() {
    let array = [0];
    let base_ptr = array.as_ptr();
    let oob_ptr = base_ptr.wrapping_sub(1);
    // SAFETY: Very unsound operation due to out-of-bounds pointer.
    // This should trigger safety violation.
    let back_base = unsafe { oob_ptr.add(1) };
    assert_eq!(base_ptr, back_base);
}
