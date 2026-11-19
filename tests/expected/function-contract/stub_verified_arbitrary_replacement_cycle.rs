// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

struct Wrapper(u32);

impl kani::Arbitrary for Wrapper {
    fn any() -> Self {
        Wrapper(kani::any())
    }
}

impl Wrapper {
    #[kani::ensures(|result| result.0 == value)]
    fn new(value: u32) -> Self {
        Self(value)
    }
}

fn independent() -> Wrapper {
    Wrapper::new(kani::any())
}

#[kani::proof_for_contract(Wrapper::new)]
fn constructor_contract() {
    Wrapper::new(kani::any());
}

#[kani::proof]
#[kani::stub_verified(Wrapper::new)]
#[kani::stub(<Wrapper as kani::Arbitrary>::any, independent)]
fn composition() {
    let value: u32 = kani::any();
    assert!(Wrapper::new(value).0 == value);
}
