// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Regression: moving a loop-local's declaration blocks to the loop head panicked with
//! "Kani can only insert instructions after terminators that have a `target` field"
//! when the local is initialized through branching control flow (the collected init
//! group spans a `SwitchInt`). Such groups are now left in place.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn pick(x: u32) -> u32 {
    x + 1
}

#[kani::proof]
fn loop_local_var_branching_init() {
    let n: u32 = kani::any_where(|x| *x <= 8);
    let mut i: u32 = 0;
    #[kani::loop_invariant(i <= n)]
    while i < n {
        // Both arms yield 1 so the loop stays provable; the `if` is only here to
        // force a branching (SwitchInt) initializer for `step`.
        let step = if i % 2 == 0 { pick(0) } else { pick(0) };
        i += step;
    }
    assert!(i == n);
}
