// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Loop contracts in CBMC backend doesn't support malloc or free in loop bodies.
//! Tracked in: https://github.com/model-checking/kani/issues/3587

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

type Data = u8;

#[kani::proof]
fn box_harness() {
    let mut i: u8 = 0;

    let mut data: Box<Data> = Box::new(0);

    #[kani::loop_invariant(*data == i)]
    while i < 10 {
        i += 1;
        data = Box::new(i);
    }
}
