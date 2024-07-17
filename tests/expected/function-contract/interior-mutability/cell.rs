// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

use std::cell::{Cell, RefCell, UnsafeCell};
use std::mem::transmute;
use std::panic;

#[allow(dead_code)]
pub struct UnsafeCellHack<T: ?Sized> {
    value: T,
}

#[allow(dead_code)]
pub struct CellHack<T: ?Sized> {
    value: UnsafeCellHack<T>,
}

#[allow(dead_code)]
pub struct RefCellHack<T: ?Sized> {
    borrow: CellHack<isize>,
    borrowed_at: CellHack<Option<&'static crate::panic::Location<'static>>>,
    value: UnsafeCellHack<T>,
}

trait ToHack<T> {
    unsafe fn hack(&self) -> &T;
}

impl<T> ToHack<UnsafeCellHack<T>> for UnsafeCell<T> {
    unsafe fn hack(&self) -> &UnsafeCellHack<T> {
        transmute(self)
    }
}

impl<T> ToHack<CellHack<T>> for Cell<T> {
    unsafe fn hack(&self) -> &CellHack<T> {
        transmute(self)
    }
}

impl<T> ToHack<RefCellHack<T>> for RefCell<T> {
    unsafe fn hack(&self) -> &RefCellHack<T> {
        transmute(self)
    }
}

// ---------------------------------------------------

struct InteriorMutability {
    x: Cell<u32>,
}

#[kani::requires(unsafe{x.x.hack()}.value.value < 100)]
#[kani::modifies(&x.x.hack().value.value)]
#[kani::ensures(|_| unsafe{x.x.hack()}.value.value < 101)]
fn modify(x: &InteriorMutability) {
    x.x.set(x.x.get() + 1)
}

#[kani::proof_for_contract(modify)]
fn main() {
    let x: InteriorMutability = InteriorMutability { x: Cell::new(kani::any()) };
    modify(&x)
}
