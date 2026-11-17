// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn add_one(x: u32) -> u32 {
    x + 1
}

#[test]
fn add_one_is_correct() {
    assert_eq!(add_one(1), 2);
}
