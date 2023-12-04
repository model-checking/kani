// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::requires(v.len() > 0)]
#[kani::modifies(&v[0])]
#[kani::ensures(v[0] == src)]
fn modify(v: &mut Vec<u32>, src: u32) {
    v[0] = src
}

#[kani::unwind(10)]
#[kani::proof_for_contract(modify)]
fn main() {
    let v_len = kani::any_where(|i| *i < 4);
    let mut v: Vec<u32> = vec![kani::any()];
    for _ in 0..v_len {
        v.push(kani::any());
    }
    modify(&mut v, kani::any());
}

#[kani::unwind(10)]
#[kani::proof]
#[kani::stub_verified(modify)]
fn modify_replace() {
    let v_len : usize = kani::any_where(|i| *i < 4);
    let mut v: Vec<u32> = Vec::with_capacity(v_len + 1);
    v.push(kani::any());
    let mut compare = Vec::with_capacity(v_len);
    for _ in 0..v_len {
        let elem = kani::any();
        v.push(elem);
        compare.push(elem);
    }
    let src = kani::any();
    modify(&mut v, src);
    kani::assert(
        v[0] == src,
        "element set"
    );
    kani::assert(
        v[1..] == compare[..],
        "vector tail equality"
    );
}
