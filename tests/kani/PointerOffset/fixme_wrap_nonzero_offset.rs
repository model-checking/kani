// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Testcase for issue [#1150](https://github.com/model-checking/kani/issues/1150) that shows
//! bug with wrapping offset operations in Kani.

/// This harness shows the issue with the current implementation of wrapping offset.
///
/// Invoking `wrapping_byte_offset` should return a pointer that is different from the original
/// pointer if the offset value is not 0.
/// See issue [#1150](https://github.com/model-checking/kani/issues/1150).
#[kani::proof]
fn fixme_incorrect_wrapping_offset() {
    let ptr: *const u8 = &0u8;
    let offset = kani::any_where(|v: &isize| *v != 0);
    let new_ptr = ptr.wrapping_byte_offset(offset);
    assert_ne!(ptr, new_ptr, "Expected new_ptr to be different than ptr");
}
