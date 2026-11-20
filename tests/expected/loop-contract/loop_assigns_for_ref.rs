// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check the use of loop_modifies for Rust's ref

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn main() {
    let mut i = 0;
    let mut a: [u8; 20] = kani::any();
    #[kani::loop_invariant(i <= 20)]
    #[kani::loop_modifies(&i, &a)]
    while i < 20 {
        a[i] = 1;
        i = i + 1;
    }
}
