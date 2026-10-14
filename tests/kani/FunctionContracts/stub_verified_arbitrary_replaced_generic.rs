// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

struct Wrapper<T>(T);

impl<T: kani::Arbitrary + Copy + Eq> kani::Arbitrary for Wrapper<T> {
    fn any() -> Self {
        Wrapper::new(kani::any())
    }
}

impl<T: Copy + Eq> Wrapper<T> {
    #[kani::ensures(|result| result.0 == value)]
    fn new(value: T) -> Self {
        Self(value)
    }
}

fn independent<T: kani::Arbitrary + Copy + Eq>() -> Wrapper<T> {
    Wrapper(kani::any())
}

#[kani::proof_for_contract(Wrapper::new)]
fn constructor_contract() {
    Wrapper::new(kani::any::<u8>());
    Wrapper::new(kani::any::<u64>());
}

// One generic replacement must work for both concrete type arguments.
#[kani::proof]
#[kani::stub_verified(Wrapper::new)]
#[kani::stub(<Wrapper<u8> as kani::Arbitrary>::any, independent)]
fn composition() {
    let small: u8 = kani::any();
    let large: u64 = kani::any();
    assert!(Wrapper::new(small).0 == small);
    assert!(Wrapper::new(large).0 == large);
}
