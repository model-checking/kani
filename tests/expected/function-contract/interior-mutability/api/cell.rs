// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Mutating Cell via as_ptr in contracts

use std::cell::Cell;

// This struct is contains Cell which can be mutated
struct InteriorMutability {
    x: Cell<u32>,
}

// contracts need to access im.x internal data through the api im.x.as_ptr
#[kani::requires(im.x.get() < 100)]
#[kani::modifies(im.x.as_ptr())]
#[kani::ensures(|_| im.x.get() < 101)]
fn modify(im: &InteriorMutability) {
    //im is an immutable reference with interior mutability
    // valid rust methodology for getting and setting value without breaking encapsulation
    im.x.set(im.x.get() + 1)
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let im: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    modify(&im)
}
