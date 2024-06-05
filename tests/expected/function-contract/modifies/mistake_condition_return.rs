// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Provide an example where users might get confuse on how to constrain
//! the return value of functions when writing function contracts.
//! In this case, users must remember that when using contracts as
//! verified stubs, the return value will be havoced. To retrict the return
//! value of a function, users may use the `result` keyword in their
//! ensures clauses.

#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
// In this case, one may think that by assuming `*ptr == 100`, automatically
// we can assume the return value of this function will also be equal to 100.
// However, contract instrumentation will create a separate non-deterministic
// value to return in this function that can only be constrained by using the
// `result` keyword. Thus the correct condition would be
// `#[kani::ensures(|result| result == 100)]`.
#[kani::ensures(|result| *ptr == 100)]
fn modify(ptr: &mut u32) -> u32 {
    *ptr += 1;
    *ptr
}

fn use_modify(ptr: &mut u32) {
    *ptr = 99;
    let res = modify(ptr);
    // This assertion won't hold because the return
    // value of `modify` is unconstrained.
    assert!(res == 100);
}

#[kani::proof]
#[kani::stub_verified(modify)]
fn harness() {
    let mut i = kani::any();
    use_modify(&mut i);
}
