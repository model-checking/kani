// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the behavior of Kani's `--no-codegen` option

#[kani::proof]
fn main() {
    let x: u8 = kani::any();
    if x < 100 {
        assert!(x < 101);
    } else {
        assert!(x > 99);
    }
}
