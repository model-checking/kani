// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Attaching the internal `CheckedSizeOfIntrinsic` marker to a function with an
//! incompatible signature should produce a diagnostic instead of an internal
//! compiler error. See https://github.com/model-checking/kani/issues/4589.

enum BadRet {
    None,
}

#[kanitool::fn_marker = "CheckedSizeOfIntrinsic"]
fn fake_checked_size_of(ptr: *const u8) -> BadRet {
    let _ = ptr;
    BadRet::None
}

#[kani::proof]
fn check() {
    let p = &0u8 as *const u8;
    let _ = fake_checked_size_of(p);
}
