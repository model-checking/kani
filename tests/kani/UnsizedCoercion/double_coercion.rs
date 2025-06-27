// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion when using box of box.
//! Tests are broken down into different crates to ensure that the reachability works for each case.

mod defs;
use defs::*;
use std::boxed::Box;

#[kani::proof]
fn check_double_coercion() {
    let id = kani::any();
    let inner: Box<Box<dyn Identity>> = Box::new(Box::new(Inner { id }));
    assert_eq!(inner.id(), id.into());
    assert_eq!(id_from_coerce(*inner), id.into());
}
