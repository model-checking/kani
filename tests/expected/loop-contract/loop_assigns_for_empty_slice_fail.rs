// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that an empty `loop_modifies` target does not allow any write: the loop names the empty
//! slice `&mut a[..0]` but writes `a[0]`, so the assigns check for that write must fail.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn main() {
    let mut a: [u8; 4] = kani::any();
    let mut i: usize = 0;
    #[kani::loop_invariant(i <= 1)]
    #[kani::loop_modifies(&i, &mut a[..0])]
    while i < 1 {
        a[i] = 0;
        i += 1;
    }
}
