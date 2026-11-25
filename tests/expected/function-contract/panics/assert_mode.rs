// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! When a contract is checked as assertions around a call (the callee is neither verified nor
//! replaced), `panics_if` is evaluated on entry, before the body, whatever the order of the
//! attributes.

#[kani::ensures(|_| true)]
#[kani::ensures(|_| true)]
#[kani::panics_if(*x == 0)]
fn reset(x: &mut u8) {
    if *x == 0 {
        *x = 1;
        return;
    }
    *x -= 1;
}

#[kani::proof]
fn check_two_ensures_then_panics_if() {
    let mut x: u8 = kani::any();
    reset(&mut x);
}

#[kani::ensures(|_| true)]
#[kani::panics_if(*x == 0)]
fn reset1(x: &mut u8) {
    if *x == 0 {
        *x = 1;
        return;
    }
    *x -= 1;
}

#[kani::proof]
fn check_one_ensures_then_panics_if() {
    let mut x: u8 = kani::any();
    reset1(&mut x);
}

#[kani::panics_if(*x == 0)]
#[kani::ensures(|_| true)]
#[kani::ensures(|_| true)]
fn reset2(x: &mut u8) {
    if *x == 0 {
        *x = 1;
        return;
    }
    *x -= 1;
}

#[kani::proof]
fn check_panics_if_then_two_ensures() {
    let mut x: u8 = kani::any();
    reset2(&mut x);
}

// For a nonzero input the body may decrement `x` to zero. The condition was evaluated on entry,
// so returning is fine.
#[kani::proof]
fn check_two_ensures_then_panics_if_nonzero() {
    let mut x: u8 = kani::any_where(|x| *x != 0);
    reset(&mut x);
}

#[kani::proof]
fn check_one_ensures_then_panics_if_nonzero() {
    let mut x: u8 = kani::any_where(|x| *x != 0);
    reset1(&mut x);
}

#[kani::proof]
fn check_panics_if_then_two_ensures_nonzero() {
    let mut x: u8 = kani::any_where(|x| *x != 0);
    reset2(&mut x);
}
