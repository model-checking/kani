// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let x = if kani::any() { 5 } else { 9 };
    if x > 10 {
        assert!(x != 11);
    }
}
