// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! test in top crate.

#[kani::proof]
fn check_toplevel_proof() {
    assert!(1 == 1);
}
