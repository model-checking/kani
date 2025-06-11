// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z function-contracts

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::requires(x < 10)]
fn func(x: usize) -> Option<usize> {
    let mut i: usize = 0;
    let mut r: usize = x;
    let N: usize = 7;
    #[kani::loop_invariant((i <= N && i >=0) && (on_entry(i) == 0 && prev(i) < 7 && prev(i) + 1 == i )  ) ]
    while i < N {
        i = i + 1;
    }
    if r + i >= x + N { Some(r) } else { None }
}

#[kani::proof]
fn harness() {
    let a = kani::any_where(|x: &usize| *x < 10);
    kani::assert(func(a).is_some(), "func(a) is some");
}
