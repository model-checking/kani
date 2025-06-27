// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

struct S<'a> {
    distraction: usize,
    target: &'a mut u32,
}
#[kani::requires(*s.target < 100)]
#[kani::modifies(s.target)]
fn modify(s: S) {
    *s.target += 1;
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn main() {
    let mut i = kani::any_where(|i| *i < 100);
    let i_copy = i;
    let s = S { distraction: 0, target: &mut i };
    modify(s);
    kani::assert(i == i_copy + 1, "Increment havocked");
}
