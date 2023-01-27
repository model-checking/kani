// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --solver kissat

//! Checks that the solver option overrides the solver attribute

#[kani::proof]
#[kani::solver("non_existing_solver")]
fn check_solver_option() {
    let v = vec![kani::any(), 3];
    let v_copy = v.clone();
    assert_eq!(v, v_copy);
}
