// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check loop in generic function

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn sum<T: Clone>(a: &[T]) -> u32
where
    u32: std::convert::From<T>,
{
    let mut j: u32 = 0;
    let mut i: usize = 0;
    #[kani::loop_invariant(i<=10 && j <= (u8::MAX as u32) * (i as u32))]
    while i < 10 {
        j = j + std::convert::Into::<u32>::into(a[i].clone());
        i = i + 1;
    }
    j
}

#[kani::proof]
fn main() {
    let a: [u8; 10] = kani::any();
    let j = sum(a.as_slice());
    assert!(j <= 2560);
}
