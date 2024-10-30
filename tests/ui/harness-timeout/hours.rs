// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness-timeout 24h -Zunstable-options
//
// This test checks that Kani accepts a `--harness-timeout` specified in hours

#[kani::proof]
fn check_harness_timeout_hours() {
    assert_ne!(42, 17);
}
