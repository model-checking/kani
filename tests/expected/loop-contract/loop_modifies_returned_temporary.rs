// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A borrowed temporary that a `return` in the loop body leaves live (`x + 0`) is added to the
//! write set of an explicit `#[kani::loop_modifies]`. Its storage has ended when an iteration
//! goes back to the loop head, where the write set is havocked; that havoc must not fail as a
//! write to a dead object.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn same(a: &u8, b: &u8) -> bool {
    *a == *b
}

fn find(a: &[u8; 8], x: u8) -> Option<usize> {
    let mut i: usize = 0;
    #[kani::loop_invariant(i <= 8)]
    #[kani::loop_modifies(&i)]
    while i < 8 {
        let s = &a[i];
        if same(s, &(x + 0)) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[kani::proof]
fn main() {
    let a: [u8; 8] = kani::any();
    let x: u8 = kani::any();
    if let Some(i) = find(&a, x) {
        assert!(a[i] == x);
    }
}
