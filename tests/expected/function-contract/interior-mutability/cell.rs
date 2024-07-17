// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

use std::cell::{Cell, UnsafeCell};
use std::mem::transmute;

trait ToHack<T : ?Sized> {
    unsafe fn hack(&self) -> &T;
}

impl<T : ?Sized> ToHack<T> for UnsafeCell<T> {
    unsafe fn hack(&self) -> &T {
        transmute(self)
    }
}

impl<T : ?Sized> ToHack<T> for Cell<T> {
    unsafe fn hack(&self) -> &T {
        transmute(self)
    }
}

// ---------------------------------------------------

struct InteriorMutability {
    x: Cell<u32>,
}

#[kani::requires(*unsafe{x.x.hack()} < 100)]
#[kani::modifies(x.x.hack())]
#[kani::ensures(|_| *unsafe{x.x.hack()} < 101)]
fn modify(x: &InteriorMutability) {
    x.x.set(x.x.get() + 1)
}

#[kani::proof_for_contract(modify)]
fn main() {
    let x: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    modify(&x)
}
