// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! When checking the contract of a function F, other functions' contract
//! clauses in the harness's call graph may themselves call F (here:
//! `make_positive`'s postcondition calls `get`, while `get` is the target of
//! proof_for_contract). Such calls must be dispatched to F's contract
//! *replacement*, not its contract *check*: they must neither consume the
//! single top-level contract check nor be write-set-checked in the clause's
//! context. See https://github.com/model-checking/kani/issues/... (clause
//! dispatch) and diffblue/cbmc#9149 (sequential top-level calls).

#[derive(Copy, Clone)]
struct Wrapper {
    v: i32,
}

impl Wrapper {
    #[kani::ensures(|result| **result == self.v)]
    fn get(&self) -> &i32 {
        &self.v
    }
}

// The postcondition evaluates `result.get()`, calling the function whose
// contract is under verification in the harness below.
#[kani::requires(v > 0)]
#[kani::ensures(|result| *result.get() == v)]
fn make_positive(v: i32) -> Wrapper {
    Wrapper { v }
}

#[kani::proof_for_contract(Wrapper::get)]
fn check_get() {
    let v: i32 = kani::any();
    kani::assume(v > 0);
    let w = make_positive(v);
    let _ = w.get();
}
