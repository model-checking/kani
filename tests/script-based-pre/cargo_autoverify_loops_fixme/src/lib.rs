// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zautomatic-harnesses

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

// Test that automatic harnesses terminate on functions with loops.

// Since foo()'s arguments implement Arbitrary, we will attempt to verify it,
// and enter an infinite loop.
// Instead, we should just skip this function, perhaps printing a message to the user that we skipped it.
fn infinite_loop() {
    loop {}
}

// Shouldn't skip this function -- it has a loop, but since it also has a loop contract,
// we can generate a contract harness for it and be assured that the proof will terminate.
fn has_loop_contract() {
    let mut x: u8 = kani::any_where(|i| *i >= 2);

    #[kani::loop_invariant(x >= 2)]
    while x > 2 {
        x = x - 1;
    }

    assert!(x == 2);
}
