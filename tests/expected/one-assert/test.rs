// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
pub fn check_assert() {
    let x: u8 = kani::any();
    let y = x;
    assert!(x == y);
}
