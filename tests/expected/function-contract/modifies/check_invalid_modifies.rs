// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that Kani reports the correct error message when modifies clause
//! includes objects of types that do not implement `kani::Arbitrary`.
//! This restriction is only applied when using contracts as verified stubs.

#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
fn modify(ptr: &mut u32) -> &'static str {
    *ptr += 1;
    let msg: &'static str = "done";
    msg
}

fn use_modify(ptr: &mut u32) {
    *ptr = 99;
    assert!(modify(ptr) == "done");
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn harness() {
    let mut i = kani::any_where(|x| *x < 100);
    use_modify(&mut i);
}
