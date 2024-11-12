// Copyright rustc Contributors
// Adapted from rust std: https://github.com/rust-lang/rust/blob/master/library/core/src/slice/memchr.rs#L38
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// kani-flags: -Z loop-contracts

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn memchar_naive_harness() {
    let text = [1, 2, 3, 4, 5];
    let x = 5;
    let mut i = 0;
    let mut r: Option<usize> = None;

    #[kani::loop_invariant(i <= text.len() && ((i == 0) || text[i-1]!= x))]
    while i < text.len() {
        if text[i] == x {
            r = Some(i);
            break;
        }

        i += 1;
    }

    assert_eq!(r, Some(4));
}
