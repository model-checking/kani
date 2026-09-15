// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

//! Test that Kani detects the cycle between a verified stub and the `Arbitrary`
//! implementation of its return type, and reports it at compile time.
//!
//! A contract replacement havocs its own return value with `kani::any::<Ret>()`.
//! Here `<Wrapper as Arbitrary>::any` calls `Wrapper::new`, which calls the
//! stubbed `Wrapper::normalize`, so the replacement re-enters itself through
//! `Arbitrary::any`. Without this check, CBMC unwinds the recursion until it
//! exhausts memory.
//!
//! See https://github.com/model-checking/kani/pull/4571

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

    fn process(self) -> u64 {
        self.normalize().value * 2
    }
}

// This `Arbitrary` implementation calls `normalize`, which closes the cycle.
impl kani::Arbitrary for Wrapper {
    fn any() -> Self {
        Wrapper::new(kani::any())
    }
}

#[kani::proof_for_contract(Wrapper::normalize)]
fn check_normalize_contract() {
    Wrapper { value: kani::any() }.normalize();
}

#[kani::proof]
#[kani::stub_verified(Wrapper::normalize)]
fn check_process_with_stub() {
    let w: Wrapper = kani::any();
    let result = w.process();
    assert!(result <= LIMIT * 2);
}
