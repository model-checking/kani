// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let x: u32 = kani::any_where(|val| *val == 5);
    if x > 3 {
        assert!(x > 4);
    }
}
