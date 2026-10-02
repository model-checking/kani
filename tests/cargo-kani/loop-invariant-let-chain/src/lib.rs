// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that `#[kani::loop_invariant]` accepts `while` loops whose condition
//! is a let chain (https://github.com/model-checking/kani/issues/4943).

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn chain_basic() {
    let mut remaining: usize = kani::any();
    kani::assume(remaining <= 4);
    #[kani::loop_invariant(remaining <= 4)]
    while remaining > 0 && let Some(next) = remaining.checked_sub(1) {
        remaining = next;
    }
    assert!(remaining <= 4);
}
