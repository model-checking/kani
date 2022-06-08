// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This is testing two behaviors:
//! 1. Hey, look, we're finding a proof harness under `tests/`
//! 2. And since it's built with crate-type=bin, we have a "dependency" on the base lib
//!    that might fail to resolve if we're not generating 'rlib' files correctly.

use cargo_tests_dir::ONE; // trigger dependency resolution

#[kani::proof]
fn check_import() {
    assert!(ONE == 1);
}
