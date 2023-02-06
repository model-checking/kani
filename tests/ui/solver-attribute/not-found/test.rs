// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that Kani errors out if specified solver binary is not found

#[kani::proof]
#[kani::solver(custom = "non_existing_solver")]
fn check() {}
