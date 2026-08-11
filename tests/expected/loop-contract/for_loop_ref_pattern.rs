// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check for-loop invariant for a loop whose pattern destructures the
//! iterator element through a reference, e.g. `for (i, &p) in ...`.
//! Such patterns introduce compiler-generated deref temporaries in the
//! pattern-binding block. Those temporaries used to be assigned via
//! `Rvalue::CopyForDeref`, which rust-lang/rust#145513 turned into plain
//! copies, and the loop-contract transformation has to keep their
//! assignments in place instead of treating them like pattern bindings
//! (see https://github.com/model-checking/kani/issues/4658).
//! This is a regression test for the spurious "dereference failure"
//! checks Kani used to emit for this pattern; it is modelled after
//! `number_of_digits_decimal_left_shift` in the Rust standard library.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

const TABLE: [u8; 16] = [5, 2, 5, 1, 2, 5, 6, 2, 5, 3, 1, 2, 5, 1, 5, 6];

fn lookup(digits: &[u8; 16], num_digits: usize, a: usize, b: usize) -> usize {
    let num_new_digits: usize = 2;
    let pow5 = &TABLE[a..];

    #[kani::loop_invariant(num_new_digits > 1)]
    for (i, &p5) in pow5.iter().enumerate().take(b - a) {
        if i >= num_digits {
            return num_new_digits - 1;
        } else if digits[i] == p5 {
            continue;
        } else if digits[i] < p5 {
            return num_new_digits - 1;
        } else {
            return num_new_digits;
        }
    }
    num_new_digits
}

#[kani::proof]
fn check_ref_pattern_deref_temp() {
    let digits: [u8; 16] = kani::any();
    let num_digits: usize = kani::any_where(|x| *x <= 16);
    let a: usize = kani::any_where(|x| *x < 8);
    let b: usize = kani::any_where(|x| *x >= a && *x <= 16);
    let n = lookup(&digits, num_digits, a, b);
    assert!(n <= 2);
}
