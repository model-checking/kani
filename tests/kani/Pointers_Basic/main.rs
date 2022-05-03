// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let x = 3;
    let y = &x;
    let mut z = *y;

    assert!(z == 3);
}
