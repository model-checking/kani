// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn repeat_const() {
    let s = String::from("a").repeat(1);
    assert_eq!(s.chars().nth(0).unwrap(), 'a');
}

#[kani::proof]
fn repeat_panic() {
    let x = String::new().repeat(1);
    let z = x.chars().nth(1).unwrap();
}
