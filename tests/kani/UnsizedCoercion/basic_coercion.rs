// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion from using built-in references and pointers.
//! Tests are broken down into different crates to ensure that the reachability works for each case.

mod defs;
use defs::*;

#[kani::proof]
fn check_base_coercion() {
    let id = kani::any();
    let inner = Inner { id };
    // Coerce `&Inner` to `&dyn Identity` which means we add a vtable ref to a fat pointer.
    assert_eq!(id_from_dyn(&inner), id.into());
}
