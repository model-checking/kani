// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let list = [1, 2, 3];
    let slice = &list[1..2];
    assert!(slice.len() > 0);
}
