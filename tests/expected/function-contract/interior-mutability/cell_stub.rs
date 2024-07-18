// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// The objective of this test is to show that the contracts for double can be replaced as a stub within the contracts for quadruple.
// This shows that we can generate kani::any() for Cell safely by breaking encapsulation

// ---------------------------------------------------
//        Abstraction Breaking Functionality
// ---------------------------------------------------

use std::cell::Cell;
use std::mem::transmute;

/// This exposes the underlying representation so that it can be added into a modifies clause within kani
trait Exposeable<T: ?Sized> {
    unsafe fn expose(&self) -> &T;
}

// This unsafe manipulation is valid due to Cell having the same underlying data layout as its internal T as explained here: https://doc.rust-lang.org/stable/std/cell/struct.Cell.html#memory-layout
impl<T: ?Sized> Exposeable<T> for Cell<T> {
    unsafe fn expose(&self) -> &T {
        transmute(self)
    }
}

// ---------------------------------------------------
//                      Test Case
// ---------------------------------------------------

// This struct is contains Cell which can be mutated
struct InteriorMutability {
    x: Cell<u32>,
}

#[kani::ensures(|_| old(*unsafe{im.x.expose()} + *unsafe{im.x.expose()}) == *unsafe{im.x.expose()})]
#[kani::requires(*unsafe{im.x.expose()} < 100)]
#[kani::modifies(im.x.expose())]
fn double(im: &InteriorMutability) {
    im.x.set(im.x.get() + im.x.get())
}

#[kani::proof_for_contract(double)]
fn double_harness() {
    let im: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    double(&im);
}

#[kani::ensures(|_| old(*unsafe{im.x.expose()} + *unsafe{im.x.expose()} + *unsafe{im.x.expose()} + *unsafe{im.x.expose()}) == *unsafe{im.x.expose()})]
#[kani::requires(*unsafe{im.x.expose()} < 50)]
#[kani::modifies(im.x.expose())]
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
