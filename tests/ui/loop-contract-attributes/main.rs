// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z loop-contracts
#![feature(stmt_expr_attributes, proc_macro_hygiene)]
extern crate core;

#[kani::proof]
fn invalid_attributes() {
    #[kani::loop_invariant(true)]
    1;
    #[kani::loop_modifies()]
    2;
    #[kani::loop_decreases(0u8)]
    3;
    #[kani::loop_invariant(1 +)]
    loop {
        break;
    }
}
