// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks the subslice pattern

#[kani::proof]
fn main() {
    let arr = [1, 2, 3];
    // s is a slice (&[i32])
    let [s @ ..] = &arr[1..];
    assert!(s[0] == 2);
    assert!(s[1] == 3);
}
