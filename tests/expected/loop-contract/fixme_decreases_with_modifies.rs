// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! FIXME: Decreases clause combined with loop_modifies fails because the
//! assigns clause checking conflicts with the decreases instrumentation.
//! Tracked in: https://github.com/model-checking/kani/issues/3168

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn fixme_decreases_with_modifies_harness() {
    let mut i: u8 = 0;
    let mut a: [u8; 5] = [0; 5];

    #[kani::loop_invariant(i <= 5)]
    #[kani::loop_modifies(&i, &a)]
    #[kani::loop_decreases(5 - i)]
    while i < 5 {
        a[i as usize] = 1;
        i += 1;
    }

    assert!(i == 5);
}
