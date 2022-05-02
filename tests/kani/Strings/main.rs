// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn test1() {
    let str = "foo";
    let string = str.to_string();
    assert!(str.chars().nth(1) == Some('o'));
    assert!(string.chars().nth(1) == Some('o'));
    assert!(string.len() == 3);
}

#[kani::proof]
fn main() {
    test1();
}
