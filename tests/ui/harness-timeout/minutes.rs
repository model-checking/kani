// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness-timeout 1m -Zunstable-options
//
// This test checks that Kani accepts a `--harness-timeout` specified in minutes

#[kani::proof]
fn check_harness_timeout_minutes() {
    let s = String::from("Hello, world!");
    assert_eq!(s.len(), 13);
}
