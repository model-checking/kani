// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests the `std::mem::swap` function using various function types.

use std::mem;

#[derive(PartialEq, Copy, Clone)]
pub struct Pair {
    value: u8,
    key: u16,
}

unsafe impl kani::Invariant for Pair {
    fn is_valid(&self) -> bool {
        true
    }
}

fn test<T: kani::Invariant + std::cmp::PartialEq + Clone>() {
    let mut var1 = kani::any::<T>();
    let mut var2 = kani::any::<T>();
    let old_var1 = var1.clone();
    let old_var2 = var2.clone();
    mem::swap(&mut var1, &mut var2);
    assert_eq!(var1, old_var2);
    assert_eq!(var2, old_var1);
}

#[kani::proof]
#[kani::unwind(9)]
fn main() {
    test::<i32>();
    test::<char>();
    test::<u32>();
    test::<[u8; 4]>();
    test::<[u16; 4]>();
    test::<Pair>();
}
