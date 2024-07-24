// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// The objective of this test is to check the modification of a RefCell used as interior mutability in an immutable struct
use std::cell::RefCell;

/// This struct is contains Cell which can be mutated
struct InteriorMutability {
    x: RefCell<u32>,
}

#[kani::requires(unsafe{*im.x.as_ptr()} < 100)]
#[kani::modifies(&im.x)]
#[kani::ensures(|_| unsafe{*im.x.as_ptr()} < 101)]
///im is an immutable reference with interior mutability
fn modify(im: &InteriorMutability) {
    im.x.replace_with(|&mut old| old + 1);
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let im: InteriorMutability = InteriorMutability { x: RefCell::new(kani::any()) };
    modify(&im)
}
