// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --solver foo=bar

//! Checks that `--solver` rejects an invalid argument

#[kani::proof]
fn check_solver_option() {}
