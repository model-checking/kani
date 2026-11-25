// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn plain(x: u8) -> u8 {
    x
}

#[cfg(kani)]
#[kani::proof]
fn check_plain() {
    assert!(plain(1) == 1);
}
