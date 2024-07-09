// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! https://doc.rust-lang.org/src/alloc/slice.rs.html#489

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(ptr_sub_ptr)]

use std::ptr;
pub fn repeat(slice: &[u16], n: usize) -> Vec<u16>
{
    if n == 0 {
        return Vec::new();
    }

    let capacity = slice.len().checked_mul(n).expect("capacity overflow");
    let mut buf = Vec::with_capacity(capacity);

    buf.extend(slice);
    {
        let mut m = n >> 1;
        // #[kani::loop_invariant(1 == 1)]
        while m > 0 {
            unsafe {
                ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    (buf.as_mut_ptr() as *mut u16).add(buf.len()),
                    buf.len(),
                );
                let buf_len = buf.len();
                buf.set_len(buf_len * 2);
            }

            m >>= 1;
        };
    }

    let rem_len = capacity - buf.len();
    if rem_len > 0 {
        unsafe {
            ptr::copy_nonoverlapping(
                buf.as_ptr(),
                (buf.as_mut_ptr() as *mut u16).add(buf.len()),
                rem_len,
            );
            buf.set_len(capacity);
        }
    }
    buf
}

#[kani::proof]
#[kani::solver(kissat)]
fn main() {
    let mut a = [1; 20];
    let n: usize = 30;
    unsafe {
        repeat(&mut a, n);
    }
}
