// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

struct Wrapper(u32);

impl kani::Arbitrary for Wrapper {
    fn any() -> Self {
        generate()
    }
}

fn generate() -> Wrapper {
    constructor_input()
}

fn constructor_input() -> Wrapper {
    Wrapper::new(kani::any())
}

impl Wrapper {
    #[kani::ensures(|result| result.0 == value)]
    fn new(value: u32) -> Self {
        Self(value)
    }
}

fn independent() -> Wrapper {
    Wrapper(kani::any())
}

#[kani::proof_for_contract(Wrapper::new)]
fn constructor_contract() {
    Wrapper::new(kani::any());
}

// Break the cycle below Arbitrary::any, through a helper's replacement.
#[kani::proof]
#[kani::stub_verified(Wrapper::new)]
#[kani::stub(constructor_input, independent)]
fn composition() {
    let value: u32 = kani::any();
    assert!(Wrapper::new(value).0 == value);
}
