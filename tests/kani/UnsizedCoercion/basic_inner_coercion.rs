// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion from using built-in references and pointers.
//! Tests are broken down into different crates to ensure that the reachability works for each case.

mod defs;
use defs::*;

#[kani::proof]
fn check_inner_dyn_coercion() {
    let inner_id = kani::any();
    let outer_id = kani::any();
    // Generate a fat pointer with a pointer to the vtable for `impl Identity for Inner`.
    let outer: &Outer<dyn Identity> = &Outer { inner: Inner { id: inner_id }, outer_id };
    // Create a new fat pointer to inner using the metadata from outer fat pointer.
    assert_eq!(id_from_dyn(&outer.inner), inner_id.into());
}
