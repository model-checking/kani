// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Attaching the internal `CheckedSizeOfIntrinsic` marker to a function whose single
//! argument is not a raw pointer should produce a diagnostic instead of an internal
//! compiler error. See https://github.com/model-checking/kani/issues/4589.

#[kanitool::fn_marker = "CheckedSizeOfIntrinsic"]
fn fake_checked_size_of(val: usize) -> Option<usize> {
    let _ = val;
    None
}

#[kani::proof]
fn check() {
    let _ = fake_checked_size_of(0);
}
