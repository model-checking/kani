// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that arithmetic operations with overflow compilation fails.
//! The compiler crashes today: https://github.com/model-checking/kani/issues/2121

#![feature(core_intrinsics)]
use std::intrinsics::sub_with_overflow;

#[kani::proof]
pub fn check_zst_sub_with_overflow() {
    let n = sub_with_overflow((), ());
    assert!(!n.1);
}
