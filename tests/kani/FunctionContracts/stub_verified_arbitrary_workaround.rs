// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts -Z stubbing
//
//! Demonstrates that stub_verified works correctly even when the Arbitrary
//! implementation calls the stubbed function, using the standalone proof
//! pattern as an alternative approach.

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

// Arbitrary calls new() which calls normalize()
impl kani::Arbitrary for Wrapper {
    fn any() -> Self {
        Wrapper::new(kani::any())
    }
}

// Step 1: Verify the contract
#[kani::proof_for_contract(Wrapper::normalize)]
fn check_normalize_contract() {
    Wrapper { value: kani::any() }.normalize();
}

// Step 2: Use stub_verified — works even though Arbitrary calls normalize,
// thanks to the ARBITRARY_NESTING_DEPTH mechanism.
#[kani::proof]
#[kani::stub_verified(Wrapper::normalize)]
fn check_process_with_stub() {
    let w: Wrapper = kani::any();
    let result = w.process();
    assert!(result <= LIMIT * 2);
}

// Alternative: standalone proof without stub_verified
#[kani::proof]
fn check_process_standalone() {
    let w: Wrapper = kani::any();
    let result = w.process();
    assert!(result <= LIMIT * 2);
}
