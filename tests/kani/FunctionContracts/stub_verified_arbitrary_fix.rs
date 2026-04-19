// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts -Z stubbing
//
//! Regression test: stub_verified no longer causes infinite recursion when
//! the type's Arbitrary implementation calls the stubbed function.
//! See: docs/dev/stub-verified-arbitrary.md

const LIMIT: u64 = 1000;

#[derive(Clone, Copy)]
struct Wrapper {
    value: u64,
}

impl Wrapper {
    #[kani::ensures(|result: &Self| result.value <= LIMIT)]
    fn normalize(self) -> Self {
        if self.value > LIMIT { Wrapper { value: LIMIT } } else { self }
    }

    fn new(v: u64) -> Self {
        Wrapper { value: v }.normalize()
    }
}

// Arbitrary calls new() which calls normalize() — the stubbed function
impl kani::Arbitrary for Wrapper {
    fn any() -> Self {
        Wrapper::new(kani::any())
    }
}

#[kani::proof_for_contract(Wrapper::normalize)]
fn check_contract() {
    Wrapper { value: kani::any() }.normalize();
}

// This previously caused infinite recursion because stub_verified replaced
// normalize globally, including inside Arbitrary::any().
// The fix: kani::any() sets ARBITRARY_NESTING_DEPTH, and the contract
// REPLACE arm falls back to the original body when the depth is > 0.
#[kani::proof]
#[kani::stub_verified(Wrapper::normalize)]
fn check_caller_with_stub() {
    let w: Wrapper = kani::any();
    assert!(w.normalize().value <= LIMIT);
}
