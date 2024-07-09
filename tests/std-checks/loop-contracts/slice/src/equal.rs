// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! https://github.com/rust-lang/rust/blob/7c4ac0603e9ee5295bc802c90575391288a69a8a/library/core/src/slice/cmp.rs#L58

fn equal(s: &[u16], other: &[u16]) -> bool {
    if s.len() != other.len() {
        return false;
    }

    for idx in 0..s.len() {
        if s[idx] != other[idx] {
            return false;
        }
    }

    true
}

#[kani::proof]
fn main() {
    let mut a = [1; 20];
    let mut b = [1; 20];
    assert!(equal(&a, &b));
}
