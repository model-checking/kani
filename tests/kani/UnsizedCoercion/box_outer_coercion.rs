// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion when using boxes.
//! Tests are broken down into different crates to ensure that the reachability works for each case.
mod defs;
use defs::*;

#[kani::proof]
fn check_outer_coercion() {
    let inner_id = kani::any();
    let outer_id = kani::any();
    let outer: Box<dyn Identity> = Box::new(Outer { inner: Inner { id: inner_id }, outer_id });
    assert_eq!(id_from_coerce(outer) >> 8, outer_id.into());
}
