// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a loop with several `#[kani::loop_invariant]` attributes has the conjunction of
//! their invariants as its invariant, for `while` and `for` loops: every invariant is checked
//! (the `*_false` harnesses must fail on entry, whichever invariant is false), and every invariant
//! can be used after the loop (the other harnesses verify).
//! A `while` loop used to keep only the last invariant, and a `for` loop used to keep only the
//! first one (or, with the attributes after the invariant carried over, failed to compile).

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn while_first_false() {
    let mut i: u8 = 0;
    let c: u8 = 0;
    #[kani::loop_invariant(c == 1)]
    #[kani::loop_invariant(i <= 3)]
    while i < 3 {
        i += 1;
    }
}

#[kani::proof]
fn while_second_false() {
    let mut i: u8 = 0;
    let c: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_invariant(c == 1)]
    while i < 3 {
        i += 1;
    }
}

#[kani::proof]
fn for_first_false() {
    let a = [1u8, 2, 3];
    let c: u8 = 0;
    #[kani::loop_invariant(c == 1)]
    #[kani::loop_invariant(true)]
    for _x in a {}
}

#[kani::proof]
fn for_second_false() {
    let a = [1u8, 2, 3];
    let c: u8 = 0;
    #[kani::loop_invariant(true)]
    #[kani::loop_invariant(c == 1)]
    for _x in a {}
}

#[kani::proof]
fn while_both_needed() {
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_invariant(j == i)]
    while i < 3 {
        i += 1;
        j += 1;
    }
    assert!(j == 3);
}

#[kani::proof]
fn while_with_prev_in_second() {
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 3)]
    #[kani::loop_invariant(prev(i) < i)]
    while i < 3 {
        i += 1;
    }
    assert!(i == 3);
}

#[kani::proof]
fn for_both_needed() {
    let a: [u8; 3] = kani::any();
    let mut s: u32 = 0;
    let mut n: u32 = 0;
    #[kani::loop_invariant(s <= kani::index as u32 * 255)]
    #[kani::loop_invariant(n == kani::index as u32)]
    for x in a {
        s += x as u32;
        n += 1;
    }
    assert!(n == 3 && s <= 765);
}
