// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --version --verbose
//
// This test validates that the --version --verbose flag works correctly
// and includes rustc version information.

#[kani::proof]
fn test_version_verbose() {
    // This is a simple proof that should always pass
    assert!(true);
}
