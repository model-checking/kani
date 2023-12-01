// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

struct S<'a> {
    distraction: &'a mut u32,
    target: &'a mut u32,
}
#[kani::requires(*s.target < 100)]
#[kani::modifies(s.target)]
#[kani::ensures(*s.target == prior + 1)]
fn modify(s: S, prior: u32) {
    *s.target += 1;
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn main() {
    let mut i = kani::any_where(|i| *i < 100);
    let i_copy = i;
    let mut distraction = kani::any();
    let distraction_copy = distraction;
    let s = S { distraction: &mut distraction, target: &mut i };
    modify(s, i_copy);
    kani::assert(i == i_copy + 1, "Increment");
    kani::assert(distraction == distraction_copy, "Unchanged");
}
