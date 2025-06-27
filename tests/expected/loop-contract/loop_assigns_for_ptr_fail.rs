// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check a fail case for loop_modifies for Rust's ptr. a is not a fat ptr.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn main() {
    let mut i = 0;
    let mut a: [u8; 20] = kani::any();
    #[kani::loop_invariant(i <= 20)]
    #[kani::loop_modifies(&i as *const _, a.as_ptr())]
    while i < 20 {
        a[i] = 1;
        i = i + 1;
    }
}
