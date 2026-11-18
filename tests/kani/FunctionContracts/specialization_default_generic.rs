// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// compile-flags: --edition 2021
// kani-flags: -Zfunction-contracts -Zstubbing

#![feature(min_specialization)]

// A method-local FnMut parameter, as in kani#4908 / kani#5135.
trait Probe {
    fn apply<F: FnMut(&u8, &u8) -> bool>(v: &mut [u8], less: &mut F);
}

impl<T> Probe for T {
    #[inline(never)]
    #[kani::requires(v.len() == 2)]
    #[kani::ensures(|_| v[0] <= v[1])]
    #[kani::modifies(v)]
    default fn apply<F>(v: &mut [u8], _less: &mut F)
    where
        F: FnMut(&u8, &u8) -> bool,
    {
        if v[1] < v[0] {
            v.swap(0, 1);
        }
    }
}

impl Probe for u8 {
    #[kani::requires(v.len() == 2)]
    #[kani::modifies(v)]
    #[kani::ensures(|_| v[0] == 9 && v[1] == 9)]
    fn apply<F: FnMut(&u8, &u8) -> bool>(v: &mut [u8], _less: &mut F) {
        v[0] = 9;
        v[1] = 9;
    }
}

#[kani::proof_for_contract(<u16 as Probe>::apply)]
#[kani::unwind(3)]
fn check_default_generic() {
    let mut v: [u8; 2] = kani::any();
    <u16 as Probe>::apply(&mut v, &mut |a, b| a < b);
}

#[kani::proof]
#[kani::stub_verified(<u16 as Probe>::apply)]
fn stub_default_generic() {
    let mut v = [1u8, 0];
    <u16 as Probe>::apply(&mut v, &mut |a, b| a < b);
    assert!(v[0] <= v[1]);
}

#[kani::proof_for_contract(<u8 as Probe>::apply)]
fn check_override() {
    let mut v: [u8; 2] = kani::any();
    <u8 as Probe>::apply(&mut v, &mut |a, b| a < b);
}

#[kani::proof]
#[kani::stub_verified(<u8 as Probe>::apply)]
fn stub_override() {
    let mut v = [1u8, 0];
    <u8 as Probe>::apply(&mut v, &mut |a, b| a < b);
    assert_eq!(v, [9, 9]);
}
