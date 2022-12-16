// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The --verbose will print the commands executed by Kani before invoking them.

#[kani::proof]
pub fn dummy_harness() {
    assert!(true);
}
