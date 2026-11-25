// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

#[kani::ensures(|result| *result)]
fn f() -> bool {
    true
}

fn g() -> bool {
    false
}

#[kani::proof_for_contract(f)]
#[kani::stub(f, g)]
fn check_contract() {
    f();
}

#[kani::proof]
#[kani::stub_verified(f)]
#[kani::stub(crate::f, g)]
fn check_verified() {
    f();
}
