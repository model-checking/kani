// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// The objective of this test is to check the modification of a Cell used as interior mutability in an immutable struct
use std::cell::Cell;

/// This struct contains Cell which can be mutated
struct InteriorMutability {
    x: Cell<u32>,
}

#[kani::requires(im.x.get() < 100)]
#[kani::modifies(&im.x)]
#[kani::ensures(|_| im.x.get() < 101)]
/// `im` is an immutable reference with interior mutability
fn modify(im: &InteriorMutability) {
    im.x.set(im.x.get() + 1)
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let im: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    modify(&im)
}
