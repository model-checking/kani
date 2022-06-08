// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn yet_another_check() {
    let x: u16 = kani::any();
    let y = x;
    assert_eq!(y - x, 0);
}
