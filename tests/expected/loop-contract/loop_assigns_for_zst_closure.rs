// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Regression test for https://github.com/model-checking/kani/issues/4786
//! Naming a zero-sized (capture-free) closure as a `loop_modifies` target used
//! to make CBMC abort with `l2_rename_rvalues case 'struct' not handled`.
//! A ZST target has a single inhabitant and occupies zero bytes, so it is
//! dropped from the assigns clause; verification should succeed.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn count_kept(a: &mut [u8], mut keep: impl FnMut(u8) -> bool) -> usize {
    let n = a.len();
    let mut i = 0;
    let mut kept = 0;
    #[kani::loop_invariant(i <= n && kept <= i)]
    #[kani::loop_modifies(&i, &kept, &keep, &raw mut *a)]
    while i < n {
        if keep(a[i]) {
            kept += 1;
        } else {
            a[i] = 0;
        }
        i += 1;
    }
    kept
}

#[kani::proof]
#[kani::unwind(9)]
fn check() {
    let mut arr: [u8; 8] = kani::any();
    // A capture-free closure is a zero-sized type.
    let k = count_kept(&mut arr, |b| b % 2 == 0);
    assert!(k <= 8);
}
