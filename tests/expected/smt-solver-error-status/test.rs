// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression test for issue #4519.
//! Ensures that Kani properly handles the ERROR status returned by CBMC
//! when using SMT solvers (z3, bitwuzla, cvc5) instead of panicking with:
//! "unknown variant `ERROR`, expected one of `FAILURE`, `COVERED`, ..."

#[kani::proof]
#[kani::solver(bitwuzla)]
pub fn check_smt_solver_error_status() {
    assert!(false);
}
