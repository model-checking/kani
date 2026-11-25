// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check that Kani identifies UB when bytes written by `write_bytes` are read as an invalid value,
//! and only then.
#![feature(core_intrinsics)]

/// Writing bytes that may not be a valid `char` is not UB by itself.
#[kani::proof]
pub fn check_invalid_write() {
    let mut val = 'a';
    let ptr = &mut val as *mut char;
    unsafe { std::intrinsics::write_bytes(ptr, kani::any(), 1) };
}

/// Reading them back as a `char` is UB if they aren't a valid one.
#[kani::proof]
pub fn check_invalid_write_then_read() {
    let mut val = 'a';
    let ptr = &mut val as *mut char;
    unsafe { std::intrinsics::write_bytes(ptr, kani::any(), 1) };
    let _c = val;
}

/// Nothing is written when `count` is 0, c.f. <https://github.com/model-checking/kani/issues/5190>.
#[kani::proof]
pub fn check_zero_count() {
    let mut buf = [std::mem::MaybeUninit::<char>::uninit(); 2];
    unsafe { std::intrinsics::write_bytes(buf.as_mut_ptr() as *mut char, kani::any(), 0) };
}

#[kani::proof]
pub fn check_valid_write() {
    let mut val = 10u128;
    let ptr = &mut val as *mut _;
    unsafe { std::intrinsics::write_bytes(ptr, kani::any(), 1) };
}
