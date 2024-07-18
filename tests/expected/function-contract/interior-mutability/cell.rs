// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

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

// contracts need to access im.x internal data through the unsafe function im.x.expose()
#[kani::requires(*unsafe{im.x.expose()} < 100)]
#[kani::modifies(im.x.expose())]
#[kani::ensures(|_| *unsafe{im.x.expose()} < 101)]
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
