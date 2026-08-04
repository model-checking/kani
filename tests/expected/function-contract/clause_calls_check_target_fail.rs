// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Companion to clause_calls_check_target.rs: ensure that the clause-context
//! dispatch of calls to the verification target does NOT weaken the actual
//! contract check performed for the harness's top-level call. The
//! postcondition of `get` below is wrong, and verification must fail even
//! though `make_positive`'s postcondition also calls `get` (which dispatches
//! to the original body in clause context).

#[derive(Copy, Clone)]
struct Wrapper {
    v: i32,
}

impl Wrapper {
    #[kani::ensures(|result| **result == self.v.wrapping_add(1))]
    fn get(&self) -> &i32 {
        &self.v
    }
}

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
