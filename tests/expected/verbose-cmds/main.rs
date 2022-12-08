// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --verbose

// `--verbose` causes Kani to print out commands before running them
// In `expected` you will find substrings of these commands for easy maintenence.
#[kani::proof]
fn real_harness() {
    assert_eq!(1 + 1, 2);
}
