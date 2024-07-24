// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// This is a valid but not recommended alternative to havocing Cell
/// Only done for test purposes

/// ---------------------------------------------------
///        Abstraction Breaking Functionality
/// ---------------------------------------------------
use std::cell::Cell;
use std::mem::transmute;

/// This exposes the underlying representation so that it can be added into a modifies clause within kani
trait Exposeable<T: ?Sized> {
    unsafe fn expose(&self) -> &T;
}

/// This unsafe manipulation is valid due to Cell having the same underlying data layout as its internal T as explained here: https://doc.rust-lang.org/stable/std/cell/struct.Cell.html#memory-layout
impl<T: ?Sized> Exposeable<T> for Cell<T> {
    unsafe fn expose(&self) -> &T {
        transmute(self)
    }
}

/// ---------------------------------------------------
///                      Test Case
/// ---------------------------------------------------

#[kani::requires(*unsafe{x.expose()} < 100)]
#[kani::modifies(x.expose())]
#[kani::ensures(|_| *unsafe{x.expose()} < 101)]
fn modify(x: &Cell<u32>) {
    // valid rust methodology for getting and setting value without breaking encapsulation
    x.set(x.get() + 1)
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let x = Cell::new(kani::any());
    modify(&x)
}
