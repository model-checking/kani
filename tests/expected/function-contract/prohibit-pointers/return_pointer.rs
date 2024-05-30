// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::ensures(|result| unsafe{ *result == *input })]
fn return_pointer(input: *const usize) -> *const usize {
    input
}

#[kani::proof_for_contract(return_pointer)]
fn return_ptr_harness() {
    let input: usize = 10;
    let input_ptr = &input as *const usize;
    unsafe {
        assert!(*(return_pointer(input_ptr)) == input);
    }
}
