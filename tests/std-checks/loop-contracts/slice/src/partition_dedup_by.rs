// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! https://doc.rust-lang.org/src/core/slice/mod.rs.html#3244-3346

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(ptr_sub_ptr)]

use std::ptr;
use std::mem;
pub fn partition_dedup_by(slice: &mut [u16]) {
    let len = slice.len();
    if len <= 1 {
        return;
    }

    let ptr = slice.as_mut_ptr();
    let mut next_read: usize = 1;
    let mut next_write: usize = 1;
    unsafe {
        // #[kani::loop_invariant(next_read <= len && next_read >= next_write && next_write >= 1)]
        while next_read < len {
            let ptr_read = ptr.add(next_read);
            let prev_ptr_write = ptr.add(next_write - 1);
            if *ptr_read != *prev_ptr_write {
                if next_read != next_write {
                    let ptr_write = prev_ptr_write.add(1);
                    mem::swap(&mut *ptr_read, &mut *ptr_write);
                }
                next_write += 1;
            }
            next_read += 1;
        };
    }
}

#[kani::proof]
#[kani::solver(kissat)]
fn main() {
    let mut a = [1; 20];
    unsafe {
        partition_dedup_by(&mut a);
    }
}
