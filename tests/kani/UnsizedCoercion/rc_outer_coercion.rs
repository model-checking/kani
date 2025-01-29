// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion from using reference counter.
//! Tests are broken down into different crates to ensure that the reachability works for each case.
mod defs;
use defs::*;
use std::rc::Rc;

#[kani::proof]
fn check_outer_coercion() {
    let inner_id = kani::any();
    let outer_id = kani::any();
    let outer: Rc<dyn Identity> = Rc::new(Outer { inner: Inner { id: inner_id }, outer_id });
    assert_eq!(id_from_coerce(outer) >> 8, outer_id.into());
}
