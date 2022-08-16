// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! test in sub-crate.

#[kani::proof]
fn check_libcrate_proof() {
    assert!(1 == 1);
}
