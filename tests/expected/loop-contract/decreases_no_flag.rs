// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// NOTE: No -Z loop-contracts flag — loop_decreases should be silently ignored.

//! Verify that `#[kani::loop_decreases]` is ignored when `-Z loop-contracts`
//! is not passed. The proof should succeed via unwinding without any
//! decreases-related checks.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
#[kani::unwind(25)]
fn check_decreases_ignored_without_flag() {
    let mut i: u32 = 20;
    #[kani::loop_decreases(i)]
    while i > 0 {
        i -= 1;
    }
    assert!(i == 0);
}
