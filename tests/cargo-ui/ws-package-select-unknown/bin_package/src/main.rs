// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {}

#[kani::proof]
fn harness_in_bin_package() {
    assert!(1 + 1 == 2);
}
