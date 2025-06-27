// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

use std::cell::UnsafeCell;

/// This struct contains UnsafeCell which can be mutated
struct InteriorMutability {
    x: UnsafeCell<u32>,
}

#[kani::requires(unsafe{*im.x.get()} < 100)]
#[kani::modifies(im.x.get())]
#[kani::ensures(|_| unsafe{*im.x.get()} < 101)]
/// `im` is an immutable reference with interior mutability
fn modify(im: &InteriorMutability) {
    unsafe { *im.x.get() += 1 }
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let im: InteriorMutability = InteriorMutability { x: UnsafeCell::new(kani::any()) };
    modify(&im)
}
