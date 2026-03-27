// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check decreases clause with an arithmetic expression (n - i).

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn decreases_expr_harness() {
    let n: u8 = kani::any_where(|i| *i >= 1 && *i <= 10);
    let mut i: u8 = 0;

    #[kani::loop_invariant(i <= n)]
    #[kani::loop_decreases(n - i)]
    while i < n {
        i += 1;
    }

    assert!(i == n);
}
