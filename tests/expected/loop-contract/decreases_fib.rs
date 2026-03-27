// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Fibonacci-style loop with decreases clause.
//! Inspired by Prusti's fib.rs test.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn fib(n: u32) -> u32 {
    let mut k = n;
    let mut a: u32 = 1;
    let mut b: u32 = 1;

    #[kani::loop_invariant(a >= 1 && b >= 1 && k <= n)]
    #[kani::loop_decreases(k)]
    while k > 2 {
        let tmp = a.wrapping_add(b);
        b = a;
        a = tmp;
        k -= 1;
    }

    a
}

#[kani::proof]
fn fib_harness() {
    let n: u32 = kani::any_where(|i| *i >= 1 && *i <= 10);
    let result = fib(n);
    assert!(result >= 1);
}
