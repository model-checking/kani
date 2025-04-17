// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check the use of "true" in loop invariant

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn main(){
    let mut i = 100;
    #[kani::loop_invariant(true)]
    while i > 1 {
        i /= 2;
    } 
}
