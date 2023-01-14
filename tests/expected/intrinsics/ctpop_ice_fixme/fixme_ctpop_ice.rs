// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that we correctly handle type mistmatch when the argument is a ZST type.
//! The compiler crashes today: https://github.com/model-checking/kani/issues/2121

#![feature(core_intrinsics)]
use std::intrinsics::ctpop;

// These shouldn't compile.
#[kani::proof]
pub fn check_zst_ctpop() {
    let n = ctpop(());
    assert!(n == ());
}
