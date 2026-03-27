// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if multi-dimensional decreases clause is correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn multi_dim_decreases_harness() {
    let n: u8 = kani::any_where(|i| *i >= 1 && *i <= 5);
    let mut i: u8 = 0;
    let mut j: u8 = 0;

    #[kani::loop_invariant(i <= n && j <= n)]
    #[kani::loop_decreases(n - i, n - j)]
    while i < n {
        if j < n {
            j += 1;
        } else {
            i += 1;
            j = 0;
        }
    }

    assert!(i == n);
}
