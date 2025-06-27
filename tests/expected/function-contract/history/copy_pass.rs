// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

struct NoCopy<T>(T);

impl<T: kani::Arbitrary> kani::Arbitrary for NoCopy<T> {
    fn any() -> Self {
        Self(kani::any())
    }
}

/// In this test, the `NoCopy` struct cannot be copied
/// to be accessed in the future, but we can still copy
/// the `u32` stored inside it to be accessed in the future.
/// The precomputation of `ptr.0` gets passed to the future.
#[kani::ensures(|result| old(ptr.0) + 1 == ptr.0)]
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
