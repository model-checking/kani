// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z unstable-options --prove-safety-only
//! Test that --prove-safety-only works

#[kani::proof]
fn div0() -> i32 {
    let x: i32 = kani::any();
    let y: i32 = kani::any();
    x / y
}
