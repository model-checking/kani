// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! Functions that panic only when their `panics_only_if` condition holds verify, and the
//! postcondition is still checked whenever they return.

#[kani::panics_only_if(x.is_none())]
#[kani::ensures(|result: &u8| Some(*result) == x)]
fn unwrap(x: Option<u8>) -> u8 {
    x.unwrap()
}

#[kani::proof_for_contract(unwrap)]
fn check_unwrap() {
    unwrap(kani::any());
}

struct Wrapper(Option<u8>);

impl Wrapper {
    #[kani::panics_only_if(self.0.is_none())]
    #[kani::ensures(|result: &u8| Some(*result) == self.0)]
    fn get(&self) -> u8 {
        self.0.unwrap()
    }
}

#[kani::proof_for_contract(Wrapper::get)]
fn check_method() {
    Wrapper(kani::any()).get();
}

/// The condition is evaluated after the precondition is assumed.
#[kani::requires(*n < 100)]
#[kani::panics_only_if(*n == 0)]
#[kani::modifies(n)]
#[kani::ensures(|_| *n == old(*n) - 1)]
fn decrement(n: &mut u8) {
    if *n == 0 {
        panic!("cannot decrement zero");
    }
    *n -= 1;
}

#[kani::proof_for_contract(decrement)]
fn check_modifies() {
    let mut n: u8 = kani::any();
    decrement(&mut n);
}

/// Panics in callees that are not replaced count as panics of the function under verification.
fn divide(x: u8) -> u8 {
    100 / x
}

#[kani::panics_only_if(x == 0)]
fn through_callee(x: u8) -> u8 {
    divide(x)
}

#[kani::proof_for_contract(through_callee)]
fn check_callee() {
    through_callee(kani::any());
}

/// `panics_only_if(true)` permits every panic but still checks the postcondition.
#[kani::panics_only_if(true)]
#[kani::ensures(|result: &u8| *result <= 10)]
fn clamp_or_panic(x: u8) -> u8 {
    assert!(x <= 200, "too big");
    if x > 10 { 10 } else { x }
}

#[kani::proof_for_contract(clamp_or_panic)]
fn check_permit_all() {
    clamp_or_panic(kani::any());
}
