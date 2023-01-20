// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that blackbox is an identity function.
use std::hint::black_box;

#[kani::proof]
fn check_blackbox() {
    // black_box is an identity function that limits compiler optimizations
    let a = 10;
    let b = black_box(a);
    assert!(a == b);
}

/// Ensure that our intrinsics code work with ZST arguments. For intrinsics, we do not ignore them.
#[kani::proof]
fn check_zst_blackbox() {
    let void = ();
    let nothing = black_box(void);
    assert!(void == nothing);
}
