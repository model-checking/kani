// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that a loop invariant can be attached to labeled `while let` and `for` loops.
//! The rewrites of these loops used to drop the label, so a `break` or `continue` that names
//! the label failed to compile.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn labeled_while_let() {
    let mut k: u8 = kani::any();
    #[kani::loop_invariant(true)]
    'outer: while let Some(n) = k.checked_sub(1) {
        k = n;
        if k == 3 {
            break 'outer;
        }
    }
    assert!(k == 0 || k == 3);
}

#[kani::proof]
fn labeled_for() {
    let a: [u8; 5] = kani::any();
    let mut sum: u32 = 0;
    #[kani::loop_invariant(sum <= (kani::index as u32 * u8::MAX as u32))]
    'outer: for x in a {
        if x == 0 {
            continue 'outer;
        }
        sum = sum + (x as u32);
    }
    assert!(sum <= 5 * 255);
}
