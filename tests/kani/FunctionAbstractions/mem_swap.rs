// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests the `std::mem::swap` function using various function types.

use std::mem;

#[derive(PartialEq, Copy, Clone)]
pub struct Pair {
    value: u8,
    key: u16,
}

impl kani::Arbitrary for Pair {
    fn any() -> Self {
        Pair { value: kani::any(), key: kani::any() }
    }
}

fn test<T: kani::Arbitrary + std::cmp::PartialEq + Clone>() {
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
