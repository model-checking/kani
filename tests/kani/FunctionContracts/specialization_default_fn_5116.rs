// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts

#![feature(min_specialization)]

trait SmallSort {
    fn small_sort(v: &mut [u8]);
}

impl<T> SmallSort for T {
    #[kani::ensures(|_| true)]
    #[kani::modifies(v)]
    default fn small_sort(v: &mut [u8]) {
        pair(v);
    }
}

impl SmallSort for u8 {
    fn small_sort(v: &mut [u8]) {
        pair(v);
    }
}

fn pair(v: &mut [u8]) {
    if v.len() == 2 && v[1] < v[0] {
        v.swap(0, 1);
    }
}

#[kani::proof_for_contract(<u16 as SmallSort>::small_sort)]
#[kani::unwind(3)]
fn check_default() {
    let mut v: [u8; 2] = kani::any();
    <u16 as SmallSort>::small_sort(&mut v);
    assert!(v[0] <= v[1]);
}
