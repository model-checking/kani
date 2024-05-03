// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that is possible to use `modifies` clause for verification, but not stubbing.
//! Here, we cover the case when the modifies clause contains references to function
//! parameters of generic types. Noticed that here the type T is not annotated with
//! `kani::Arbitrary` since this is no longer a requirement if the contract is only
//! use for verification.

pub mod contracts {
    #[kani::modifies(x)]
    #[kani::modifies(y)]
    pub fn swap<T>(x: &mut T, y: &mut T) {
        core::mem::swap(x, y)
    }
}

mod verify {
    use super::*;

    #[kani::proof_for_contract(contracts::swap)]
    pub fn check_swap_primitive() {
        let mut x: u8 = kani::any();
        let mut y: u8 = kani::any();
        contracts::swap(&mut x, &mut y)
    }
}
