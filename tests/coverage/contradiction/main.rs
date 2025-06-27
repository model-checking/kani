// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn contradiction() {
    let x: u8 = kani::any();
    let mut y: u8 = 0;
    if x > 5 {
        if x < 2 {
            y = x;
        }
    } else {
        assert!(x < 10);
    }
}
