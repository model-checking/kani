// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

fn f() -> bool {
    false
}

fn g() -> bool {
    true
}

#[kani::ensures(|result| *result)]
fn caller() -> bool {
    f()
}

#[kani::proof_for_contract(caller)]
#[kani::stub(f, g)]
fn check_caller() {
    caller();
}

#[kani::ensures(|result| *result)]
fn h() -> bool {
    true
}

#[kani::proof]
#[kani::stub_verified(h)]
#[kani::stub(f, g)]
fn check_different_verified_target() {
    assert!(h());
    assert!(f());
}

#[kani::proof_for_contract(h)]
#[kani::stub(g, h)]
fn check_reverse_contract() {
    h();
}

#[kani::proof]
#[kani::stub_verified(h)]
#[kani::stub(g, h)]
fn check_reverse_verified() {
    assert!(h());
}
