// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Positive control for the intrinsic-marker signature guard: applying the internal
//! markers to functions with the exact intrinsic signature must still generate the
//! intrinsic bodies and verify successfully.
//! See https://github.com/model-checking/kani/issues/4589.

#[kanitool::fn_marker = "CheckedSizeOfIntrinsic"]
fn fake_checked_size_of(ptr: *const u8) -> Option<usize> {
    let _ = ptr;
    None
}

#[kanitool::fn_marker = "CheckedAlignOfIntrinsic"]
fn fake_checked_align_of(ptr: *const u8) -> Option<usize> {
    let _ = ptr;
    None
}

#[kani::proof]
fn check() {
    let val = 0u8;
    let ptr = &val as *const u8;
    assert!(fake_checked_size_of(ptr) == Some(std::mem::size_of::<u8>()));
    assert!(fake_checked_align_of(ptr) == Some(std::mem::align_of::<u8>()));
}
