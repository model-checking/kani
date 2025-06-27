// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion when using boxes.
//! Tests are broken down into different crates to ensure that the reachability works for each case.

mod defs;
use defs::*;
use std::boxed::Box;

#[kani::proof]
fn check_base_coercion() {
    let id = kani::any();
    let inner: Box<dyn Identity> = Box::new(Inner { id });
    assert_eq!(id_from_coerce(inner), id.into());
}
