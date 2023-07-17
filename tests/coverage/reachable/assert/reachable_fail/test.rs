// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let x = 5;
    if kani::any() {
        kani::cover!();
        assert!(x != 5);
    }
}
