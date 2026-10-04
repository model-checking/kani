// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Same panic as loop_local_var_branching_init, with the branch condition reading
//! a variable declared before the loop. The collected init group mentions no loop
//! local, so the reads-loop-locals skip does not apply; only the terminator-shape
//! check keeps the group from being moved and panicking.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn pick(x: u32) -> u32 {
    x + 1
}

#[kani::proof]
fn loop_local_var_branching_init_external_cond() {
    let gate: bool = kani::any();
    let n: u32 = kani::any_where(|x| *x <= 8);
    let mut i: u32 = 0;
    #[kani::loop_invariant(i <= n)]
    while i < n {
        // Both arms yield 1 so the loop stays provable; the `if` is only here to
        // force a branching (SwitchInt) initializer for `step`.
        let step = if gate { pick(0) } else { pick(0) };
        i += step;
    }
    assert!(i == n);
}
