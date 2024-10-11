// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

extern crate kani;
use kani::{Arbitrary, Invariant};

#[derive(Arbitrary, Invariant)]
struct Stars {
    #[safety_constraint(*value <= 5)]
    value: u8,
}

impl Stars {
    #[kani::requires(stars <= 5)]
    #[kani::ensures(|res| res.is_safe())]
    fn new(stars: u8) -> Stars {
        Stars::try_new(stars).unwrap()
    }

    #[kani::ensures(|ret| kani::implies!(stars <= 5 => ret.is_ok()))]
    #[kani::ensures(|ret| kani::implies!(stars > 5 => ret.is_err()))]
    fn try_new(stars: u8) -> Result<Stars, ()> {
        if stars > 5 { Err(()) } else { Ok(Stars { value: stars }) }
    }
}

#[cfg(kani)]
mod verify {
    use super::*;
    kani::gen_proof_for_contract!(check_new_contract, Stars::new, 1);
    kani::gen_proof_for_contract!(check_try_new_contract, Stars::try_new, 1);
}
