// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zcontracts-as-assertions

// Test -Zcontracts-as-assertions for preconditions.

#[kani::requires(*ptr < 100)]
#[kani::ensures(|result| old(*ptr + 3) == *ptr)]
#[kani::modifies(ptr)]
fn add_three(ptr: &mut u32) {
    *ptr += 1;
    add_two(ptr);
}

#[kani::requires(*ptr < 101)]
#[kani::ensures(|_| old(*ptr + 2) == *ptr)]
fn add_two(ptr: &mut u32) {
    *ptr += 1;
    add_one(ptr);
}

#[kani::requires(*ptr == 1)]
#[kani::modifies(ptr)]
fn add_one(ptr: &mut u32) {
    *ptr += 1;
}

mod should_fail {
    use crate::*;

    // add_three and add_one's preconditions are asserted, causing failure.
    #[kani::proof]
    fn prove_add_three() {
        let mut i = kani::any();
        add_three(&mut i);
    }

    // add_three's precondition is asserted, causing failure.
    #[kani::proof_for_contract(add_one)]
    fn prove_add_one() {
        let mut i = kani::any();
        add_three(&mut i);
    }
}
mod should_pass {
    use crate::*;
    // Same as should_fail::prove_add_one, with the added assumption of add_three's precondition.
    // Note that add_one's precondition gets assumed since it's the target of the proof_for_contract.
    #[kani::proof_for_contract(add_one)]
    fn prove_add_one() {
        let mut i = kani::any();
        kani::assume(i < 100);
        add_three(&mut i);
    }
}
