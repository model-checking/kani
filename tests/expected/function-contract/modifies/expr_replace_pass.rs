// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::requires(**ptr < 100)]
#[kani::modifies(ptr.as_ref())]
#[kani::ensures(*ptr.as_ref() == prior + 1)]
fn modify(ptr: &mut Box<u32>, prior: u32) {
    *ptr.as_mut() += 1;
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn main() {
    let val = kani::any_where(|i| *i < 100);
    let mut i = Box::new(val);
    modify(&mut i, val);
    kani::assert(*i == val + 1, "Increment");
}
