// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Contracts of dependencies are asserted by default (#3802) as an aid for
//! detecting API misuse in user code. Calls made while evaluating *contract
//! clauses*, however, execute the original body (exact semantics, still fully
//! UB-checked) without re-asserting the callee's contract: clause expressions
//! are specifications, and asserting specification-level plumbing multiplies
//! verification cost without checking user code.
//!
//! `one`'s postcondition below calls `plus_one(0)`, which violates
//! `plus_one`'s (overly strict) precondition but is well-defined: the clause
//! must evaluate to true without a contract-assertion failure. The same
//! misuse in *user code* (`check_misuse`) must still be caught.

#[kani::requires(x >= 10)]
fn plus_one(x: u8) -> u8 {
    x.wrapping_add(1)
}

#[kani::ensures(|result| *result == plus_one(0))]
fn one() -> u8 {
    1
}

#[kani::proof]
fn check_clause_call_not_asserted() {
    let _ = one();
}

#[kani::proof]
fn check_misuse_still_caught() {
    let _ = plus_one(0);
}
