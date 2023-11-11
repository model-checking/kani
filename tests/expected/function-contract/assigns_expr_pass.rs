// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::requires(**ptr < 100)]
#[kani::modifies(*ptr.as_ref())]
fn modify(ptr: &mut Box<u32>) {
    *ptr.as_mut() += 1;
}

#[kani::proof_for_contract(modify)]
fn main() {
    let mut i = Box::new(kani::any());
    modify(&mut i);
}
