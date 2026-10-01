// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Jumps in the body of a loop whose invariant uses `prev` that do not leave that body keep their
//! meaning: a `break` and a `continue` of a nested loop (unlabeled, and naming the nested loop's
//! label), and a `return` from a closure defined in the body. Each harness's assertion holds.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn nested_unlabeled_break() {
    let mut k: u8 = 3;
    #[kani::loop_invariant(k <= 3 && k < prev(k))]
    while k > 0 {
        k -= 1;
        loop {
            break;
        }
    }
    assert!(k == 0);
}

#[kani::proof]
fn nested_labeled_continue_and_break() {
    let mut k: u8 = 3;
    #[kani::loop_invariant(k <= 3 && k < prev(k))]
    while k > 0 {
        k -= 1;
        let mut m: u8 = 0;
        #[kani::loop_invariant(m <= 2)]
        'inner: loop {
            m += 1;
            if m < 2 {
                continue 'inner;
            }
            break 'inner;
        }
    }
    assert!(k == 0);
}

#[kani::proof]
fn closure_return_in_body() {
    let mut k: u8 = 3;
    #[kani::loop_invariant(k <= 3 && k < prev(k))]
    while k > 0 {
        let dec = |x: u8| -> u8 {
            if x == 0 {
                return 0;
            }
            x - 1
        };
        k = dec(k);
    }
    assert!(k == 0);
}
