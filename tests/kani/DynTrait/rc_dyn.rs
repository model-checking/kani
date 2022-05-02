// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Test Rc<dyn T> to ensure it works correctly. We only expect the object
// to be created once and dropped once. Clone should have the same inner
// structure.
#![feature(ptr_metadata)]

use std::rc::Rc;

static mut COUNTER: i8 = 0;

struct Table {
    pub fancy: bool,
}

trait Furniture {
    // Instance method signature
    fn cost(&self) -> i16;
}

// Implement the Furniture for Table.
impl Furniture for Table {
    fn cost(&self) -> i16 {
        if self.fancy { 1000 } else { 200 }
    }
}

impl Table {
    pub fn new(fancy: bool) -> Self {
        unsafe {
            COUNTER += 1;
        }
        Table { fancy: fancy }
    }

    // Create a table but return Rc<dyn Furniture>.
    fn new_furniture(fancy: bool) -> Rc<dyn Furniture> {
        Rc::new(Table::new(fancy))
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        unsafe {
            COUNTER -= 1;
        }
    }
}

#[kani::proof]
fn check_rc_dyn_value() {
    let val = kani::any();
    let table = Table::new(val);
    let furniture = Table::new_furniture(val);
    assert_eq!(furniture.cost(), table.cost());
}

#[kani::proof]
fn check_rc_dyn_drop() {
    let table = Table::new_furniture(kani::any());
    let furniture = table.clone();
    unsafe {
        assert_eq!(COUNTER, 1);
    }

    drop(furniture);
    unsafe {
        assert_eq!(COUNTER, 1);
    }

    drop(table);
    unsafe {
        assert_eq!(COUNTER, 0);
    }
}

#[kani::proof]
fn check_rc_dyn_raw_parts() {
    let table = Table::new_furniture(kani::any());
    let furniture = table.clone();

    let (table_ptr, table_vtable) = Rc::as_ptr(&table).to_raw_parts();
    let (furn_ptr, furn_vtable) = Rc::as_ptr(&furniture).to_raw_parts();
    assert_eq!(table_ptr, furn_ptr);
    assert_eq!(table_vtable, furn_vtable);
}

#[kani::proof]
fn check_rc_dyn_diff_raw_parts() {
    let table = Table::new_furniture(kani::any());
    let furniture = Table::new_furniture(kani::any());

    let (table_ptr, table_vtable) = Rc::as_ptr(&table).to_raw_parts();
    let (furn_ptr, furn_vtable) = Rc::as_ptr(&furniture).to_raw_parts();

    // Check that they have different data but same vtable.
    assert_ne!(table_ptr, furn_ptr);
    assert_eq!(table_vtable, furn_vtable);

    // TODO: Enable this once fat pointer comparison has been fixed.
    // https://github.com/model-checking/kani/issues/327
    // assert_ne!(Rc::as_ptr(&table), Rc::as_ptr(&furniture));
}
