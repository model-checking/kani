// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Tests that providing the "modifies" clause havocks the pointer such
// that the increment can no longer be observed (in the absence of an
// "ensures" clause)

#[kani::requires(**ptr < 100)]
#[kani::modifies(ptr.as_ref())]
fn modify(ptr: &mut Box<u32>) {
    *ptr.as_mut() += 1;
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn main() {
    let val = kani::any_where(|i| *i < 100);
    let mut i = Box::new(val);
    modify(&mut i);
    kani::assert(*i == val + 1, "Increment");
}
