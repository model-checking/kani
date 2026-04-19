// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts -Z stubbing
//
//! Demonstrates that stub_verified works correctly when the Arbitrary
//! implementation does NOT call the stubbed function.

const LIMIT: u64 = 1000;

#[derive(Clone, Copy, kani::Arbitrary)]
struct MyType {
    value: u64,
}

impl MyType {
    #[kani::ensures(|result: &Self| result.value <= LIMIT)]
    fn normalize(self) -> Self {
        if self.value > LIMIT { MyType { value: LIMIT } } else { self }
    }

    fn process(self) -> u64 {
        self.normalize().value * 2
    }
}

// Step 1: Verify the contract
#[kani::proof_for_contract(MyType::normalize)]
fn check_normalize_contract() {
    MyType { value: kani::any() }.normalize();
}

// Step 2: Use stub_verified in a caller — works because
// kani::Arbitrary for MyType (derived) does NOT call normalize
#[kani::proof]
#[kani::stub_verified(MyType::normalize)]
fn check_process_with_stub() {
    let t: MyType = kani::any();
    let result = t.process();
    assert!(result <= LIMIT * 2);
}
