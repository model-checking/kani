// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Jumps in the body of a loop whose invariant uses `prev` that do not leave that body keep their
//! meaning: a `break` and a `continue` of a nested loop (unlabeled, and naming the nested loop's
//! label), a `break` out of a labeled block, a `return` from a closure defined in the body, and a
//! `return` and loop `break` inside a `fn` item defined in the body. Each harness's assertion holds.

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

#[kani::proof]
fn labeled_block_break() {
    let mut k: u8 = 7;
    let mut hits: u8 = 0;
    // `break 'done` leaves the block only: `hits` counts every iteration but the one with k == 5.
    #[kani::loop_invariant(k <= 7 && k < prev(k) && hits == 7 - k - (k <= 5) as u8)]
    while k > 0 {
        k -= 1;
        'done: {
            if k == 5 {
                break 'done;
            }
            hits += 1;
        }
    }
    assert!(hits == 6);
}

#[kani::proof]
fn fn_item_in_body() {
    let mut k: u8 = 3;
    #[kani::loop_invariant(k <= 3 && k < prev(k))]
    while k > 0 {
        // An item has its own `return`s and loops, which belong to it rather than to the loop.
        fn dec(x: u8) -> u8 {
            loop {
                break;
            }
            if x == 0 {
                return 0;
            }
            x - 1
        }
        k = dec(k);
    }
    assert!(k == 0);
}
