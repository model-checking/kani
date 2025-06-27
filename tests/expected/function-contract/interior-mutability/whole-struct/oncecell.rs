// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// The objective of this test is to check the modification of an OnceCell used as interior mutability in an immutable struct
use std::cell::OnceCell;

/// This struct contains OnceCell which can be mutated
struct InteriorMutability {
    x: OnceCell<u32>,
}

#[kani::requires(im.x.get().is_none())]
#[kani::modifies(&im.x)]
#[kani::ensures(|_| im.x.get().is_some())]
fn modify(im: &InteriorMutability) {
    im.x.set(5).expect("")
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let im: InteriorMutability = InteriorMutability { x: OnceCell::new() };
    modify(&im)
}
