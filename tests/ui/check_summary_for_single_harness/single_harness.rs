// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness check_foo --exact
//! Check for the summary line at the end of the verification output

#[kani::proof]
fn check_foo() {
    assert!(1 == 1);
}
