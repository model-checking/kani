// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Test that modifies a string

#[kani::requires(x == "aaa")]
#[kani::modifies(x)]
#[kani::ensures(|_| x == "AAA")]
fn to_upper(x: &mut str) {
    x.make_ascii_uppercase();
}

#[kani::proof_for_contract(to_upper)]
fn harness() {
    let mut s = String::from("aaa");
    let x: &mut str = s.as_mut_str();
    to_upper(x);
}
