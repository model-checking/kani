// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let array = [1, 2, 3, 4, 5, 6];
    let slice = &array[2..5];
    assert!(slice[0] == 3);
    assert!(slice[2] == 5);
}
