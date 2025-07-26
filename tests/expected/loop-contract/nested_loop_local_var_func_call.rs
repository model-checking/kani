// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if loop assign clause can be infered for inner-loop when there are local variables of outter-loop body.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn sum_pair(x: u32, y: u32) -> u32 {
    x + y
}

#[kani::proof]
fn main() {
    let mut i: u32 = 0;
    let mut s: u32 = 0;
    let t1 = kani::any_where(|x| *x < 5);
    let t2 = kani::any_where(|x| *x < 5);
    #[kani::loop_invariant(i <= 5 && s == i * 20)]
    while i < 5 {
        let mut j = sum_pair(t1, t2);
        let mut k = sum_pair(t2, t1);
        #[kani::loop_invariant(j <= 10 && k ==j)]
        while j < 10 {
            j = j + 1;
            k = k + 1;
        }
        s = s + j + k;
        i = i + 1;
    }
    assert!(s == 100);
}
