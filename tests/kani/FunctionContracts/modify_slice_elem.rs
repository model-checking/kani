// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani correctly verify the contract that modifies slices.
//!
//! Note that this test used to crash while parsing the annotations.
// kani-flags: -Zfunction-contracts
extern crate kani;

#[kani::requires(idx < slice.len())]
#[kani::modifies(slice.as_ptr().wrapping_add(idx))]
#[kani::ensures(|_| slice[idx] == new_val)]
fn modify_slice(slice: &mut [u32], idx: usize, new_val: u32) {
    *slice.get_mut(idx).unwrap() = new_val;
}

#[cfg(kani)]
mod verify {
    use super::modify_slice;

    #[kani::proof_for_contract(modify_slice)]
    fn check_modify_slice() {
        let mut data = kani::vec::any_vec::<u32, 5>();
        modify_slice(&mut data, kani::any(), kani::any())
    }
}
