// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts --solver minisat

/// The objective of this test is to show that the contracts for double can be replaced as a stub within the contracts for quadruple.
/// This shows that we can generate `kani::any()` for Cell.
use std::cell::Cell;

/// This struct contains Cell which can be mutated
struct InteriorMutability {
    x: Cell<u32>,
}

#[kani::ensures(|_| old(im.x.get() + im.x.get()) == im.x.get())]
#[kani::requires(im.x.get() < 100)]
#[kani::modifies(im.x.as_ptr())]
fn double(im: &InteriorMutability) {
    im.x.set(im.x.get() + im.x.get())
}

#[kani::proof_for_contract(double)]
fn double_harness() {
    let im: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    double(&im);
}

#[kani::ensures(|_| old(im.x.get() + im.x.get() + im.x.get() + im.x.get()) == im.x.get())]
#[kani::requires(im.x.get() < 50)]
#[kani::modifies(im.x.as_ptr())]
fn quadruple(im: &InteriorMutability) {
    double(im);
    double(im)
}

#[kani::proof_for_contract(quadruple)]
#[kani::stub_verified(double)]
fn quadruple_harness() {
    let im: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    quadruple(&im);
}
