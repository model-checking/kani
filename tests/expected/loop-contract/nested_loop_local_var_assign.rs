// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check if loop assign clause can be infered for inner-loop when there are local variables of outter-loop body.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn func(x: usize) {
    let mut j = 0;
    const CHUNK_SIZE: usize = 32;
    #[kani::loop_invariant(j <=x )]
    while j < x {
        let mut i = 0;
        let mut sum = 0_usize;
        #[kani::loop_invariant(i <= CHUNK_SIZE && prev(i) <= CHUNK_SIZE && prev(i) + 1 == i && sum <= CHUNK_SIZE && on_entry(sum) + i >= sum )]
        while i < CHUNK_SIZE {
            sum += kani::any::<bool>() as usize;
            i += 1;
        }

        j += 1;
    }
}

#[kani::proof]
fn harness() {
    let _ = func(10);
}
