// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Test Rc<dyn T> to ensure it works correctly. We only expect the object
// to be created once and dropped once. Clone should have the same inner
// structure.

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
}

impl Drop for Table {
    fn drop(&mut self) {
        unsafe {
            COUNTER -= 1;
        }
    }
}

// Create a table but return Rc<dyn Furniture>.
fn table(fancy: bool) -> Rc<dyn Furniture> {
    Rc::new(Table::new(fancy))
}

#[kani::proof]
fn check_rc_dyn() {
    let table = table(kani::any());
    let furniture = table.clone();
    unsafe {
        assert_eq!(COUNTER, 1);
    }
    assert_eq!(table.cost(), furniture.cost());

    /* TODO: Finish this.
    let (table_ptr, table_vtable) = table.to_raw_parts();
    let (furn_ptr, furn_vtable) = furn.to_raw_parts();
    assert_eq!(table_ptr, furn_ptr);
    assert_eq!(table_vtable, furn_vtable);
    */
    drop(furniture);
    unsafe {
        assert_eq!(COUNTER, 1);
    }

    drop(table);
    unsafe {
        assert_eq!(COUNTER, 0);
    }
}
