// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z loop-contracts
#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[rustfmt::skip]
#[kani::proof]
fn match_decreases_detects_stale_measure() {
    let mut n = 3u8;
    let measure = n;
    match true {
        true =>
        #[kani::loop_decreases(measure)]
        #[kani::loop_modifies(&n)]
        #[kani::loop_invariant(n <= 3)]
        while n > 0 { n -= 1; },
        false => (),
    }
}
