// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[derive(Clone)]
struct NoCopy<T>(T);

impl<T: kani::Arbitrary> kani::Arbitrary for NoCopy<T> {
    fn any() -> Self {
        Self(kani::any())
    }
}

#[kani::remember(ptr.clone())]
#[kani::ensures(|result||old_ptr : &NoCopy<u32>| (*old_ptr).0 + 1 == ptr.0)]
#[kani::requires(ptr.0 < 100)]
#[kani::modifies(&mut ptr.0)]
fn modify(ptr: &mut NoCopy<u32>) {
    ptr.0 += 1;
}

#[kani::proof_for_contract(modify)]
fn main() {
    let mut i = kani::any();
    modify(&mut i);
}
