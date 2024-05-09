// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::cell::RefCell;
use std::ops::Deref;

#[kani::modifies(cell.borrow().deref())]
fn modifies_ref_cell(cell: &RefCell<u32>) {
    *cell.borrow_mut() = 100;
}

#[kani::proof_for_contract(modifies_ref_cell)]
fn check_harness() {
    let rc = RefCell::new(0);
    modifies_ref_cell(&rc);
}
