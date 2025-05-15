// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that `kani autoharness --list` finds all of the manual and automatic harnesses
// and correctly matches them to their target function.
// Note that the proof_for_contract attributes use different, but equivalent, paths to their target functions;
// this tests that we can group the harnesses under the same target function even when the attribute value strings differ.

fn f_u8(x: u8) -> u8 {
    x
}

#[kani::requires(x != 0 && y != 0)]
#[kani::ensures(|result : &u8| *result != 0 && x % *result == 0 && y % *result == 0)]
#[kani::recursion]
fn has_recursion_gcd(x: u8, y: u8) -> u8 {
    let mut max = x;
    let mut min = y;
    if min > max {
        let val = max;
        max = min;
        min = val;
    }

    let res = max % min;
    if res == 0 { min } else { has_recursion_gcd(min, res) }
}

#[kani::proof_for_contract(crate::has_recursion_gcd)]
fn my_harness() {
    has_recursion_gcd(kani::any(), kani::any());
}

#[kani::proof_for_contract(has_recursion_gcd)]
fn my_harness_2() {
    has_recursion_gcd(kani::any(), kani::any());
}

mod verify {
    #[kani::requires(x != 0 && y != 0)]
    #[kani::ensures(|result : &u8| *result != 0 && x % *result == 0 && y % *result == 0)]
    #[kani::recursion]
    fn has_recursion_gcd(x: u8, y: u8) -> u8 {
        let mut max = x;
        let mut min = y;
        if min > max {
            let val = max;
            max = min;
            min = val;
        }

        let res = max % min;
        if res == 0 { min } else { has_recursion_gcd(min, res) }
    }

    #[kani::proof_for_contract(crate::verify::has_recursion_gcd)]
    fn my_harness() {
        has_recursion_gcd(kani::any(), kani::any());
    }

    #[kani::proof_for_contract(has_recursion_gcd)]
    fn my_harness_2() {
        has_recursion_gcd(kani::any(), kani::any());
    }
}
