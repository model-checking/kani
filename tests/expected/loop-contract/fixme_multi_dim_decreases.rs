// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! FIXME: CBMC does not support multi-dimensional (tuple) decreases clauses
//! when passed through Kani's `#spec_decreases` irep annotation. The tuple
//! expression is emitted but goto-instrument does not perform lexicographic
//! comparison on it — only single integer expressions work.
//! Tracked in: https://github.com/model-checking/kani/issues/3168

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn fixme_multi_dim_decreases_harness() {
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
