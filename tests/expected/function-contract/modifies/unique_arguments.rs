// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::modifies(a)]
#[kani::modifies(b)]
#[kani::ensures(|result| *a == 1)]
#[kani::ensures(|result| *b == 2)]
fn two_pointers(a: &mut u32, b: &mut u32) {
    *a = 1;
    *b = 2;
}

#[kani::proof_for_contract(two_pointers)]
fn test_contract() {
    two_pointers(&mut kani::any(), &mut kani::any());
}

#[kani::proof]
#[kani::stub_verified(two_pointers)]
fn test_stubbing() {
    let mut a = kani::any();
    let mut b = kani::any();

    two_pointers(&mut a, &mut b);

    kani::assert(a == 1, "a is 1");
    kani::assert(b == 2, "b is 2");
}
