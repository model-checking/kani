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

#[kani::proof_for_contract(modify)]
fn main() {
    let mut i = kani::any();
    let s = S { distraction: 0, target: &mut i };
    modify(s);
}
