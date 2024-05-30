// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that Kani does not report any error when unused modifies clauses
//! includes objects of types that do not implement `kani::Arbitrary`.

#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
#[kani::ensures(|result| result == 100)]
fn modify(ptr: &mut u32) -> u32 {
    *ptr += 1;
    *ptr
}

#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
fn wrong_modify(ptr: &mut u32) -> &'static str {
    *ptr += 1;
    let msg: &'static str = "done";
    msg
}

fn use_modify(ptr: &mut u32) {
    *ptr = 99;
    assert!(modify(ptr) == 100);
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn harness() {
    let mut i = kani::any_where(|x| *x < 100);
    use_modify(&mut i);
}
