// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Only harnessable when both `--cfg` values survive the trip through `RUSTFLAGS`, so this fails
// with "no harnesses" if whitespace handling drops flags instead of just the empty ones.
#[cfg(all(first, second))]
#[kani::proof]
fn check_both_cfgs_arrived() {
    assert!(1 + 1 == 2);
}
