// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check a decreases clause together with a loop modifies clause.
//! The measure is a variable: a measure such as `5 - i` is computed once, before the loop, so
//! its decreases check fails until https://github.com/model-checking/kani/issues/4585 is fixed
//! (see `decreases_fail_unsupported_measure.rs`).

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn decreases_with_modifies_harness() {
    let mut i: u8 = 0;
    let mut r: u8 = 5;
    let mut a: [u8; 5] = [0; 5];

    #[kani::loop_invariant(i <= 5 && r as u16 + i as u16 == 5)]
    #[kani::loop_modifies(&i, &r, &a)]
    #[kani::loop_decreases(r)]
    while i < 5 {
        a[i as usize] = 1;
        i += 1;
        r -= 1;
    }

    assert!(i == 5);
}
