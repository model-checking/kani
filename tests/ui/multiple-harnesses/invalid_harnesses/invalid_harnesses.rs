// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness existing --harness non_existing --harness invalid
//! Check that we correctly error out if one or more selected harnesses do not exist
#[kani::proof]
fn existing() {
    assert!(1 == 1);
}

/// A harness that will fail verification if it is run.
#[kani::proof]
fn ignored_harness() {
    assert!(3 == 2);
}
