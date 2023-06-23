// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness non_existing --harness invalid
//! Check that we just ignore non-matching filters

/// A harness that will fail verification if it is run.
#[kani::proof]
fn ignored_harness() {
    assert!(3 == 2);
}
