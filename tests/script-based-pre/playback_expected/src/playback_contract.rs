// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani correctly adds tests to when the harness is a proof for contract.
extern crate kani;

#[kani::requires(idx < slice.len())]
#[kani::modifies(slice)]
#[kani::ensures(| _ | slice[idx] == new_val)]
fn modify_slice(slice: &mut [u32], idx: usize, new_val: u32) {
    // Inject bug by incrementing index first.
    let new_idx = idx + 1;
    *slice.get_mut(new_idx).expect("Expected valid index, but contract is wrong") = new_val;
}

#[kani::proof_for_contract(modify_slice)]
fn check_modify_slice() {
    let mut data: [u32; 4] = kani::any();
    modify_slice(&mut data, kani::any(), kani::any())
}
