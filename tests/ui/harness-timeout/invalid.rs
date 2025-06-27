// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness-timeout 5k -Zunstable-options
//
// This test checks the error message when the argument to the `--harness-timeout` option is invalid

#[kani::proof]
fn check_invalid_harness_timeout() {
    assert!(true);
}
